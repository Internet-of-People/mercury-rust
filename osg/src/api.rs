use std::path::PathBuf;
use std::str::FromStr;

use failure::{bail, ensure, err_msg, Fallible};
use futures::prelude::*;
use log::*;

use crate::model::*;
use crate::repo::*;
use crate::vault::{self, ProfileVault};
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

pub type ApiRes = Fallible<()>;
pub trait Api {
    fn restore_vault(&mut self, phrase: String) -> ApiRes;
    fn restore_all_profiles(&mut self) -> ApiRes;
    fn list_profiles(&self) -> ApiRes;
    fn set_active_profile(&mut self, my_profile_id: ProfileId) -> ApiRes;

    fn create_profile(&mut self) -> ApiRes;
    fn show_profile(&self, profile_id: Option<ProfileId>, kind: ProfileRepositoryKind) -> ApiRes;

    fn revert_profile(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes;
    fn publish_profile(&mut self, my_profile_id: Option<ProfileId>, force: bool) -> ApiRes;
    fn restore_profile(&mut self, my_profile_id: Option<ProfileId>, force: bool) -> ApiRes;

    fn list_incoming_links(&self, my_profile_id: Option<ProfileId>) -> ApiRes;

    fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: ProfileId,
    ) -> ApiRes;
    fn remove_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: ProfileId,
    ) -> ApiRes;
    fn set_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: AttributeId,
        value: AttributeValue,
    ) -> ApiRes;
    fn clear_attribute(&mut self, my_profile_id: Option<ProfileId>, key: AttributeId) -> ApiRes;
}

pub struct Context {
    vault_path: PathBuf,
    vault: Option<Box<ProfileVault>>,
    local_repo: FileProfileRepository, // NOTE match arms of show_profile() conflicts with Box<LocalProfileRepository>
    base_repo: Box<PrivateProfileRepository>,
    remote_repo: Box<PrivateProfileRepository>,
    explorer: Box<ProfileExplorer>,
}

// TODO !!! The current implementation assumes that though the ProfileRepository
//      shows an asynchronous interface, in reality all results are ready without waiting.
//      For real asynchronous repositories this implementation has to be changed.
impl Context {
    pub fn new(
        vault_path: PathBuf,
        vault: Option<Box<ProfileVault>>,
        local_repo: FileProfileRepository,
        base_repo: Box<PrivateProfileRepository>,
        remote_repo: Box<PrivateProfileRepository>,
        explorer: Box<ProfileExplorer>,
    ) -> Self {
        Self { vault_path, vault, local_repo, base_repo, remote_repo, explorer }
    }

    // TODO there should be no version of vault getters that panic
    /// # Panic
    /// If there is no vault given to `new`
    fn vault(&self) -> &ProfileVault {
        self.vault.as_ref().unwrap().as_ref()
    }

    /// # Panic
    /// If there is no vault given to `new`
    fn mut_vault(&mut self) -> &mut ProfileVault {
        self.vault.as_mut().unwrap().as_mut()
    }

    pub fn take_vault(&mut self) -> Option<Box<ProfileVault>> {
        self.vault.take()
    }

    fn restore_vault(&mut self, phrase: String) -> Fallible<()> {
        let old_vault_op = self.take_vault();
        ensure!(
            old_vault_op.is_none(),
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
        let profile_id_opt = my_profile_option.or_else(|| self.vault().get_active().ok()?);
        let profile_id = match profile_id_opt {
            Some(profile_id) => profile_id,
            None => bail!(
                "Command option my_profile_id is unspecified and no active default profile was found"
            ),
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
        self.mut_vault().restore_id(&profile_id)?;
        let profile = self.base_repo.get(&profile_id).wait()?;
        self.local_repo.restore(profile.clone())?;
        Ok(profile)
    }

    fn pull_base_profile(&mut self, profile_id: &ProfileId) -> ApiRes {
        debug!("Fetching remote version of profile {} to base cache", profile_id);
        let remote_profile = self.remote_repo.get(&profile_id).wait()?;
        self.base_repo.set(remote_profile).wait()
    }

    //         | none | some  (base)
    // --------+------+-----------------------------
    //    none | ok   | ok (but server impl error)
    //    some | err  | err if local.ver > base.ver
    // (local)
    fn ensure_no_local_changes(&self, profile_id: &ProfileId) -> ApiRes {
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
    fn ensure_no_remote_changes(&self, profile_id: &ProfileId) -> ApiRes {
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

    fn restore_one_profile(&mut self, profile_id: &ProfileId, force: bool) -> ApiRes {
        if force {
            debug!("Applying remote profile version, overwriting any local changes if present");
        } else {
            debug!("Applying remote profile version with conflict detection");
            self.ensure_no_local_changes(profile_id)?;
        }

        self.pull_base_profile(profile_id)?;
        self.revert_local_profile_to_base(profile_id)?;
        Ok(())
    }
}

impl Api for Context {
    fn list_profiles(&self) -> ApiRes {
        let profile_ids = self.vault().list()?;
        info!("You have {} profiles", profile_ids.len());
        let active_profile_opt = self.vault().get_active()?;
        for (i, profile_id) in profile_ids.iter().enumerate() {
            let status = match active_profile_opt {
                Some(ref active_profile) => {
                    if active_profile == profile_id {
                        " (active)"
                    } else {
                        ""
                    }
                }
                None => "",
            };
            info!("  {}: {}{}", i, profile_id, status);
        }
        Ok(())
    }

    fn list_incoming_links(&self, my_profile_id: Option<ProfileId>) -> ApiRes {
        let profile = self.selected_profile(my_profile_id)?;
        let followers = self.explorer.followers(&profile.id()).wait()?;
        info!("You have {} followers", followers.len());
        for (idx, follower) in followers.iter().enumerate() {
            info!("  {}: {:?}", idx, follower);
        }
        Ok(())
    }

    // TODO this should also work if profile is not ours and we have no control over it.
    //      Then it should consult an explorer and show all public information.
    fn show_profile(&self, profile_id: Option<ProfileId>, kind: ProfileRepositoryKind) -> ApiRes {
        // NOTE must also work with a profile that is not ours
        let profile_id = self.selected_profile_id(profile_id)?;
        use ProfileRepositoryKind::*;
        let repo = match kind {
            Local => &self.local_repo,
            Base => self.base_repo.as_ref(),
            Remote => self.remote_repo.as_ref(),
        };
        let profile = repo.get(&profile_id).wait()?;
        let public_profile = profile.public_data();
        let links = public_profile.links();
        let attributes = public_profile.attributes();

        info!("Details of profile id {}", profile_id);
        info!("Profile version: {}", public_profile.version());
        info!("  {} attributes:", attributes.len());
        for (i, attribute) in attributes.iter().enumerate() {
            info!("    {}: {:?}", i, attribute);
        }
        info!("  {} subscriptions:", links.len());
        for (i, peer_id) in links.iter().enumerate() {
            info!("    {}: {:?}", i, peer_id);
        }
        Ok(())
    }

    fn create_profile(&mut self) -> ApiRes {
        let new_profile_key = self.mut_vault().create_key()?;
        let empty_profile = PrivateProfileData::empty(&new_profile_key);
        self.local_repo.set(empty_profile).wait()?;
        info!("Created and activated profile with id {}", new_profile_key.key_id());
        Ok(())
    }

    fn revert_profile(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes {
        let profile_id = self.selected_profile(my_profile_id)?.id();
        let profile = self.revert_local_profile_to_base(&profile_id)?;
        info!("Reverted profile {} to last known remote version {}", profile_id, profile.version());
        Ok(())
    }

    fn publish_profile(&mut self, my_profile_id: Option<ProfileId>, force: bool) -> ApiRes {
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
        info!("Published profile {} to remote repository", profile_id);

        self.pull_base_profile(&profile_id)
    }

    fn restore_profile(&mut self, my_profile_id: Option<ProfileId>, force: bool) -> ApiRes {
        let profile_id = self.selected_profile_id(my_profile_id)?;
        self.restore_one_profile(&profile_id, force)?;
        info!("Successfully restored profile {}", profile_id);
        Ok(())
    }

    fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: ProfileId,
    ) -> ApiRes {
        let mut profile = self.selected_profile(my_profile_id)?;
        let link = profile.mut_public_data().create_link(&peer_profile_id);
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()?;
        debug!("Created link: {:?}", link);
        info!("Created link to peer profile {}", peer_profile_id);
        Ok(())
    }

    fn remove_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: ProfileId,
    ) -> ApiRes {
        let mut profile = self.selected_profile(my_profile_id)?;
        profile.mut_public_data().remove_link(&peer_profile_id);
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()?;
        info!("Removed link from profile {}", peer_profile_id);
        Ok(())
    }

    fn set_active_profile(&mut self, my_profile_id: ProfileId) -> ApiRes {
        self.mut_vault().set_active(&my_profile_id)?;
        info!("Active profile was set to {}", my_profile_id);
        Ok(())
    }

    fn set_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: AttributeId,
        value: AttributeValue,
    ) -> ApiRes {
        let mut profile = self.selected_profile(my_profile_id)?;
        info!("Setting attribute {} to {}", key, value);
        profile.mut_public_data().set_attribute(key, value);
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()
    }

    fn clear_attribute(&mut self, my_profile_id: Option<ProfileId>, key: AttributeId) -> ApiRes {
        let mut profile = self.selected_profile(my_profile_id)?;
        profile.mut_public_data().clear_attribute(&key);
        profile.mut_public_data().increase_version();
        self.local_repo.set(profile).wait()?;
        info!("Cleared attribute: {}", key);
        Ok(())
    }

    fn restore_vault(&mut self, phrase: String) -> ApiRes {
        self.restore_vault(phrase)?;
        self.restore_all_profiles()
    }

    fn restore_all_profiles(&mut self) -> ApiRes {
        let profiles = self.vault().profiles()?;
        let len = self.vault().len();

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

        info!("Tried {} profiles, successfully restored {}", try_count, restore_count);
        Ok(())
    }
}

pub fn generate_vault() {
    let new_bip39_phrase = keyvault::Seed::generate_bip39();
    let words = new_bip39_phrase.split(' ');
    warn!(
        r#"Make sure you back these words up somewhere safe
and run the 'restore vault' command of this application first!"#
    );
    words.enumerate().for_each(|(i, word)| info!("    {:2}: {}", i + 1, word));
}
