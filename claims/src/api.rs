use std::path::PathBuf;
use std::str::FromStr;

use failure::{bail, ensure, err_msg, Fallible};
use futures::prelude::*;
use log::*;

use crate::model::*;
use did::repo::*;
use did::vault::{self, ProfileAlias, ProfileVault};
use keyvault::PublicKey as KeyVaultPublicKey;

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum ProfileRepositoryKind {
    Local,
    Base,
    Remote, // TODO Differentiate several remotes, e.g. by including a network address here like Remote(addr)
}

impl FromStr for ProfileRepositoryKind {
    type Err = failure::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "local" => Ok(ProfileRepositoryKind::Local),
            "base" => Ok(ProfileRepositoryKind::Base),
            "remote" => Ok(ProfileRepositoryKind::Remote),
            _ => Err(err_msg("Invalid profile repository kind")),
        }
    }
}

// TODO expose alias and sync state of profile here
pub trait Api {
    fn restore_vault(&mut self, phrase: String) -> Fallible<()>;
    fn restore_all_profiles(&mut self) -> Fallible<(u32, u32)>;

    fn get_profile(
        &self,
        id: Option<ProfileId>,
        repo_kind: ProfileRepositoryKind,
    ) -> Fallible<PrivateProfileData>;
    fn list_profiles(&self) -> Fallible<Vec<(ProfileAlias, ProfileId)>>;

    fn create_profile(&mut self, alias: ProfileAlias) -> Fallible<ProfileId>;
    fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()>;
    fn get_active_profile(&self) -> Fallible<Option<ProfileId>>;

    fn revert_profile(&mut self, my_profile_id: Option<ProfileId>) -> Fallible<PrivateProfileData>;
    fn publish_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<ProfileId>;
    fn restore_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData>;

    fn list_incoming_links(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<Link>>;
    fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<Link>;
    fn remove_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<()>;
    fn set_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
        value: &AttributeValue,
    ) -> Fallible<()>;
    fn clear_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
    ) -> Fallible<()>;
}

pub struct Context {
    vault_path: PathBuf,
    vault: Option<Box<ProfileVault + Send>>,
    local_repo: FileProfileRepository, // NOTE match arms of get_profile() conflicts with Box<LocalProfileRepository>
    base_repo: Box<PrivateProfileRepository + Send>,
    remote_repo: Box<PrivateProfileRepository + Send>,
    explorer: Box<ProfileExplorer + Send>,
}

const ERR_MSG_VAULT_UNINITIALIZED: &str = "Vault is uninitialized, `restore vault` first";

// TODO !!! The current implementation assumes that though the ProfileRepository
//      shows an asynchronous interface, in reality all results are ready without waiting.
//      For real asynchronous repositories this implementation has to be changed,
//      likely by changing the API itself to be async, or maybe somehow making sure that
//      the wait() calls below run on a different thread and don't block the running reactor tasks.
impl Context {
    pub fn new(
        vault_path: PathBuf,
        vault: Option<Box<ProfileVault + Send>>,
        local_repo: FileProfileRepository,
        base_repo: Box<PrivateProfileRepository + Send>,
        remote_repo: Box<PrivateProfileRepository + Send>,
        explorer: Box<ProfileExplorer + Send>,
    ) -> Self {
        Self { vault_path, vault, local_repo, base_repo, remote_repo, explorer }
    }

    fn vault(&self) -> Fallible<&ProfileVault> {
        self.vault
            .as_ref()
            .map(|v| v.as_ref() as &ProfileVault)
            .ok_or_else(|| err_msg(ERR_MSG_VAULT_UNINITIALIZED))
    }

    fn mut_vault(&mut self) -> Fallible<&mut ProfileVault> {
        self.vault
            .as_mut()
            .map(|v| v.as_mut() as &mut ProfileVault)
            .ok_or_else(|| err_msg(ERR_MSG_VAULT_UNINITIALIZED))
    }

    pub fn save_vault(&mut self) -> Fallible<()> {
        if let Some(ref mut vault) = self.vault {
            let vault_path = self.vault_path.clone();
            vault.save(&vault_path)?;
        }
        Ok(())
    }

    fn restore_vault_impl(&mut self, phrase: String) -> Fallible<()> {
        ensure!(
            self.vault.is_none(),
            r#"You already have an active vault.
Please delete {}
before trying to restore another vault."#,
            self.vault_path.to_string_lossy()
        );

        let seed_res = keyvault::Seed::from_bip39(&phrase);
        let seed = match seed_res {
            Ok(seed) => Ok(seed),
            Err(e) => {
                if let Some(keyvault::Bip39ErrorKind::InvalidChecksum) =
                    e.find_root_cause().downcast_ref()
                {
                    Err(err_msg("All the words entered were valid, still the checksum was wrong.\nIs the order of the words correct?"))
                } else {
                    Err(e)
                }
            }
        }?;
        let new_vault = vault::HdProfileVault::create(seed);
        self.vault.replace(Box::new(new_vault));
        Ok(())
    }

    // NOTE needed besides selected_profile() because some operations do not require a profile
    //      to be present in local profile repository, operation still has to work
    fn selected_profile_id(&self, my_profile_option: Option<ProfileId>) -> Fallible<ProfileId> {
        let profile_id_opt = my_profile_option.or_else(|| self.vault().ok()?.get_active().ok()?);
        let profile_id = match profile_id_opt {
            Some(profile_id) => profile_id,
            None => bail!("ProfileId is unspecified and no active default profile was found"),
        };
        info!("Your active profile is {}", profile_id);
        Ok(profile_id)
    }

    fn selected_profile(
        &self,
        my_profile_option: Option<ProfileId>,
    ) -> Fallible<PrivateProfileData> {
        let profile_id = self.selected_profile_id(my_profile_option)?;
        let profile = self.local_repo.get(&profile_id).wait()?;
        Ok(profile)
    }

    fn revert_local_profile_to_base(
        &mut self,
        profile_id: &ProfileId,
    ) -> Fallible<PrivateProfileData> {
        self.mut_vault()?.restore_id(&profile_id)?;
        let profile = self.base_repo.get(&profile_id).wait()?;
        self.local_repo.restore(profile.clone())?;
        Ok(profile)
    }

    fn pull_base_profile(&mut self, profile_id: &ProfileId) -> Fallible<()> {
        debug!("Fetching remote version of profile {} to base cache", profile_id);
        let remote_profile = self.remote_repo.get(&profile_id).wait()?;
        self.base_repo.set(remote_profile).wait()
    }

    //         | none | some  (base)
    // --------+------+-----------------------------
    //    none | ok   | ok (but server impl error)
    //    some | err  | err if local.ver > base.ver
    // (local)
    fn ensure_no_local_changes(&self, profile_id: &ProfileId) -> Fallible<()> {
        let base_profile_res = self.base_repo.get(&profile_id).wait();
        let local_profile_res = self.local_repo.get(&profile_id).wait();

        let implementation_error = base_profile_res.is_ok() && local_profile_res.is_err();
        if implementation_error {
            return Err(local_profile_res.unwrap_err());
        }

        let profile_has_local_changes = local_profile_res.is_ok()
            && (base_profile_res.is_err()
                || local_profile_res.unwrap().version() > base_profile_res.unwrap().version());
        if profile_has_local_changes {
            // TODO do we really need an error here or just log some message and return success?
            bail!("Conflict detected: local profile was modified since last known remote version");
        }
        Ok(())
    }

    // NOTE remote server should detect version conflict updating the entry
    //         | none | some  (base)
    // --------+------+-----------------------------
    //    none | ok   | ok (but server impl error?)
    //    some | err  | err if remote.ver > base.ver
    // (remote)
    fn ensure_no_remote_changes(&self, profile_id: &ProfileId) -> Fallible<()> {
        let remote_profile_res = self.remote_repo.get(&profile_id).wait();
        let base_profile_res = self.base_repo.get(&profile_id).wait();

        let implementation_error = base_profile_res.is_ok() && remote_profile_res.is_err();
        if implementation_error {
            return Err(remote_profile_res.unwrap_err());
        }

        let profile_has_remote_changes = remote_profile_res.is_ok()
            && (base_profile_res.is_err()
                || remote_profile_res.unwrap().version() > base_profile_res.unwrap().version());
        if profile_has_remote_changes {
            // TODO do we really need an error here or just log some message and return success?
            bail!("Conflict detected: remote profile was modified since last known version");
        }
        Ok(())
    }

    fn restore_one_profile(
        &mut self,
        profile_id: &ProfileId,
        force: bool,
    ) -> Fallible<PrivateProfileData> {
        if force {
            debug!("Applying remote profile version, overwriting any local changes if present");
        } else {
            debug!("Applying remote profile version with conflict detection");
            self.ensure_no_local_changes(profile_id)?;
        }

        self.pull_base_profile(profile_id)?;
        let profile = self.revert_local_profile_to_base(profile_id)?;
        Ok(profile)
    }
}

impl Api for Context {
    fn list_profiles(&self) -> Fallible<Vec<(ProfileAlias, ProfileId)>> {
        self.vault()?.list()
    }

    fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()> {
        self.mut_vault()?.set_active(my_profile_id)?;
        Ok(())
    }

    fn get_active_profile(&self) -> Fallible<Option<ProfileId>> {
        self.vault()?.get_active()
    }

    // TODO this should also work if profile is not ours and we have no control over it.
    //      Then it should consult an explorer and show all public information.
    fn get_profile(
        &self,
        profile_id: Option<ProfileId>,
        kind: ProfileRepositoryKind,
    ) -> Fallible<PrivateProfileData> {
        // NOTE must also work with a profile that is not ours
        let profile_id = self.selected_profile_id(profile_id)?;
        use ProfileRepositoryKind::*;
        let repo = match kind {
            Local => &self.local_repo,
            Base => self.base_repo.as_ref(),
            Remote => self.remote_repo.as_ref(),
        };
        let profile = repo.get(&profile_id).wait()?;
        Ok(profile)
    }

    fn create_profile(&mut self, alias: ProfileAlias) -> Fallible<ProfileId> {
        let new_profile_key = self.mut_vault()?.create_key(alias)?;
        let empty_profile = PrivateProfileData::empty(&new_profile_key);
        self.local_repo.set(empty_profile).wait()?;
        Ok(new_profile_key.key_id())
    }

    fn revert_profile(&mut self, my_profile_id: Option<ProfileId>) -> Fallible<PrivateProfileData> {
        let profile_id = self.selected_profile(my_profile_id)?.id();
        let profile = self.revert_local_profile_to_base(&profile_id)?;
        Ok(profile)
    }

    fn publish_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<ProfileId> {
        let mut profile = self.selected_profile(my_profile_id)?;
        let profile_id = profile.id().to_owned();

        if force {
            debug!("Publishing local profile version, overwriting any remote changes if present");
            let remote_profile = self.remote_repo.get(&profile_id).wait()?;
            if remote_profile.version() >= profile.version() {
                info!("Conflicting profile version found on remote server, forcing overwrite");
                profile.mut_public_data().set_version(remote_profile.version() + 1);
                self.local_repo.set(profile.clone()).wait()?;
            }
        } else {
            debug!("Publishing local profile version with conflict detection");
            self.ensure_no_remote_changes(&profile_id)?;
        }

        self.remote_repo.set(profile).wait()?;
        self.pull_base_profile(&profile_id)?;
        Ok(profile_id)
    }

    fn restore_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData> {
        let profile_id = self.selected_profile_id(my_profile_id)?;
        let profile = self.restore_one_profile(&profile_id, force)?;
        Ok(profile)
    }

    fn list_incoming_links(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<Link>> {
        let profile = self.selected_profile(my_profile_id)?;
        let followers = self.explorer.followers(&profile.id()).wait()?;
        Ok(followers)
    }

    fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<Link> {
        let mut profile = self.selected_profile(my_profile_id)?;
        let link = profile.mut_public_data().create_link(peer_profile_id);
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()?;
        debug!("Created link: {:?}", link);
        Ok(link)
    }

    fn remove_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<()> {
        let mut profile = self.selected_profile(my_profile_id)?;
        profile.mut_public_data().remove_link(&peer_profile_id);
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()?;
        Ok(())
    }

    fn set_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
        value: &AttributeValue,
    ) -> Fallible<()> {
        let mut profile = self.selected_profile(my_profile_id)?;
        profile.mut_public_data().set_attribute(key.to_owned(), value.to_owned());
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()
    }

    fn clear_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
    ) -> Fallible<()> {
        let mut profile = self.selected_profile(my_profile_id)?;
        profile.mut_public_data().clear_attribute(key);
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()?;
        Ok(())
    }

    fn restore_vault(&mut self, phrase: String) -> Fallible<()> {
        self.restore_vault_impl(phrase)
    }

    fn restore_all_profiles(&mut self) -> Fallible<(u32, u32)> {
        let profiles = self.vault()?.profiles()?;
        let len = self.vault()?.len();

        let mut try_count = 0;
        let mut restore_count = 0;
        for idx in 0..len {
            try_count += 1;
            let profile_id = profiles.id(idx as i32)?;
            if let Err(e) = self.restore_one_profile(&profile_id, false) {
                info!("  No related data found for profile {}: {}", profile_id, e);
                continue;
            }
            restore_count += 1;
        }
        debug!("  After the known profiles, we try to look for unknown ones.");
        let mut idx = len;
        let mut end = len + vault::GAP;
        while idx < end {
            try_count += 1;
            let profile_id = profiles.id(idx as i32)?;
            if let Err(e) = self.restore_one_profile(&profile_id, true) {
                debug!("  Profile {} was tried, but not found: {}", profile_id, e);
                idx += 1;
                continue;
            }
            end = idx + vault::GAP;
            idx += 1;
            restore_count += 1;
        }

        Ok((try_count, restore_count))
    }
}
