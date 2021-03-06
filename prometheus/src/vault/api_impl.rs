use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

use failure::{bail, ensure, err_msg, format_err, Fallible};
use futures::{future::Either, prelude::*};
use log::*;
use multiaddr::Multiaddr;

use crate::daemon::NetworkState;
use crate::vault::api::*;
use crate::{DidHomeStatus, HomeNode};
use claims::claim_schema::ClaimSchemaRegistry;
pub use claims::claim_schema::{ClaimSchemas, SchemaId, SchemaVersion};
use claims::model::*;
use claims::repo::*;
use did::vault::{self, ProfileLabel, ProfileMetadata, ProfileVault, ProfileVaultRecord};
use keyvault::PublicKey as KeyVaultPublicKey;
use mercury_home_protocol::{ProfileFacets, RelationHalfProof, RelationProof};

const ERR_MSG_VAULT_UNINITIALIZED: &str = "Vault is uninitialized, `restore vault` first";

fn lock_r<T>(lock: &RwLock<T>) -> Fallible<RwLockReadGuard<'_, T>> {
    lock.try_read().map_err(|e| format_err!("Failed to lock crawler: {}", e))
}

fn lock_w<T>(lock: &RwLock<T>) -> Fallible<RwLockWriteGuard<'_, T>> {
    lock.try_write().map_err(|e| format_err!("Failed to lock crawler: {}", e))
}

pub struct VaultState {
    vault_path: PathBuf,
    schema_path: PathBuf, // TODO Re-reading all schemas each time might be expensive
    vault: Option<Arc<dyn ProfileVault + Send + Sync>>,
    local_repo: Arc<RwLock<FileProfileRepository>>, // NOTE match arms of get_profile() conflicts with Box<LocalProfileRepository>
    base_repo: Box<dyn PrivateProfileRepository + Send>,
    remote_repo: Box<dyn PrivateProfileRepository + Send>,
}

// TODO !!! The current implementation assumes that though the ProfileRepository
//      shows an asynchronous interface, in reality all results are ready without waiting.
//      For real asynchronous repositories this implementation has to be changed,
//      likely by changing the API itself to be async, or maybe somehow making sure that
//      the wait() calls below run on a different thread and don't block the running reactor tasks.
impl VaultState {
    pub fn new(
        vault_path: PathBuf,
        schema_path: PathBuf,
        vault: Option<Arc<dyn ProfileVault + Send + Sync>>,
        local_repo: Arc<RwLock<FileProfileRepository>>,
        base_repo: Box<dyn PrivateProfileRepository + Send>,
        remote_repo: Box<dyn PrivateProfileRepository + Send>,
    ) -> Self {
        Self { vault_path, schema_path, vault, local_repo, base_repo, remote_repo }
    }

    fn vault(&self) -> Fallible<Arc<dyn ProfileVault>> {
        self.vault
            .as_ref()
            .cloned()
            .map(|v| v as Arc<dyn ProfileVault>)
            .ok_or_else(|| err_msg(ERR_MSG_VAULT_UNINITIALIZED))
    }

    fn mut_vault(&mut self) -> Fallible<&mut dyn ProfileVault> {
        self.vault
            .as_mut()
            .and_then(|v| Arc::get_mut(v))
            .map(|v| v as &mut dyn ProfileVault)
            .ok_or_else(|| err_msg(ERR_MSG_VAULT_UNINITIALIZED))
    }

    pub fn save_vault(&mut self) -> Fallible<()> {
        if let Some(ref mut vault) = self.vault {
            let vault_path = self.vault_path.clone();
            vault.save(&vault_path)?;
        }
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
        let profile = lock_r(self.local_repo.as_ref())?.get(&profile_id).wait()?;
        Ok(profile)
    }

    fn revert_local_profile_to_base(
        &mut self,
        profile_id: &ProfileId,
    ) -> Fallible<PrivateProfileData> {
        self.mut_vault()?.restore_id(&profile_id)?;
        let profile = self.base_repo.get(&profile_id).wait()?;
        lock_w(self.local_repo.as_ref())?.restore(profile.clone())?;
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
        let local_profile_res = lock_r(self.local_repo.as_ref())?.get(&profile_id).wait();

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

impl VaultApi for VaultState {
    fn restore_vault(&mut self, phrase: String) -> Fallible<()> {
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
        self.vault.replace(Arc::new(new_vault));
        self.save_vault()
    }

    fn restore_all_profiles(&mut self) -> Fallible<RestoreCounts> {
        let keys = self.vault()?.keys()?;
        let len = self.vault()?.len() as u32;

        let mut try_count = 0;
        let mut restore_count = 0;
        for idx in 0..len {
            try_count += 1;
            let profile_id = keys.id(idx as i32)?;
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
            let profile_id = keys.id(idx as i32)?;
            if let Err(e) = self.restore_one_profile(&profile_id, true) {
                debug!("  Profile {} was tried, but not found: {}", profile_id, e);
                idx += 1;
                continue;
            }
            end = idx + vault::GAP;
            idx += 1;
            restore_count += 1;
        }

        self.save_vault()?;
        Ok(RestoreCounts { try_count, restore_count })
    }

    fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()> {
        self.mut_vault()?.set_active(my_profile_id)?;
        self.save_vault()
    }

    fn get_active_profile(&self) -> Fallible<Option<ProfileId>> {
        self.vault()?.get_active()
    }

    //fn list_vault_records(&self) -> Fallible<Vec<VaultEntry>> {
    fn list_vault_records(&self) -> Fallible<Vec<ProfileVaultRecord>> {
        let entries = self.vault()?.profiles()?;
        Ok(entries)
    }

    //fn create_profile(&mut self, label: Option<ProfileLabel>) -> Fallible<VaultEntry> {
    fn create_profile(&mut self, label: Option<ProfileLabel>) -> Fallible<ProfileVaultRecord> {
        // TODO label should not be parameter of create_key()
        let new_profile_key = self.mut_vault()?.create_key(label)?;
        let empty_profile = PrivateProfileData::empty(&new_profile_key);
        lock_w(self.local_repo.as_ref())?.set(empty_profile).wait()?;
        self.vault()?.profile(&new_profile_key.key_id())
    }

    fn get_vault_record(&self, my_profile_id: Option<ProfileId>) -> Fallible<ProfileVaultRecord> {
        let profile_id = self.selected_profile(my_profile_id)?.id();
        self.vault()?.profile(&profile_id)
    }

    fn set_profile_label(
        &mut self,
        my_profile_id: Option<ProfileId>,
        label: ProfileLabel,
    ) -> Fallible<()> {
        let profile_id = self.selected_profile(my_profile_id)?.id();
        self.mut_vault()?.set_label(profile_id, label)?;
        self.save_vault()
    }

    fn get_profile_metadata(&self, my_profile_id: Option<ProfileId>) -> Fallible<ProfileMetadata> {
        let profile_id = self.selected_profile(my_profile_id)?.id();
        self.vault()?.metadata_by_id(&profile_id)
    }

    fn set_profile_metadata(
        &mut self,
        my_profile_id: Option<ProfileId>,
        data: ProfileMetadata,
    ) -> Fallible<()> {
        let profile_id = self.selected_profile(my_profile_id)?.id();
        self.mut_vault()?.set_metadata(profile_id, data)?;
        self.save_vault()
    }

    // TODO this should also work if profile is not ours and we have no control over it.
    //      Then it should consult an explorer and show all public information.
    fn get_profile_data(
        &self,
        profile_id: Option<ProfileId>,
        kind: ProfileRepositoryKind,
    ) -> Fallible<PrivateProfileData> {
        use ProfileRepositoryKind::*;
        // NOTE must also work with a profile that is not ours
        let profile_id = self.selected_profile_id(profile_id)?;
        let profile = match kind {
            Local => lock_r(self.local_repo.as_ref())?.get(&profile_id),
            Base => self.base_repo.get(&profile_id),
            Remote => self.remote_repo.get(&profile_id),
        };
        profile.wait()
    }

    fn revert_profile(&mut self, my_profile_id: Option<ProfileId>) -> Fallible<PrivateProfileData> {
        let profile_id = self.selected_profile(my_profile_id)?.id();
        let profile = self.revert_local_profile_to_base(&profile_id)?;
        self.save_vault()?;
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
                lock_w(self.local_repo.as_ref())?.set(profile.clone()).wait()?;
            }
        } else {
            debug!("Publishing local profile version with conflict detection");
            self.ensure_no_remote_changes(&profile_id)?;
        }

        self.remote_repo.set(profile).wait()?;
        self.pull_base_profile(&profile_id)?;
        self.save_vault()?;
        Ok(profile_id)
    }

    fn restore_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData> {
        let profile_id = self.selected_profile_id(my_profile_id)?;
        let profile = self.restore_one_profile(&profile_id, force)?;
        self.save_vault()?;
        Ok(profile)
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
        lock_w(self.local_repo.as_ref())?.set(profile).wait()?;
        self.save_vault()
    }

    fn clear_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
    ) -> Fallible<()> {
        let mut profile = self.selected_profile(my_profile_id)?;
        profile.mut_public_data().clear_attribute(key);
        profile.mut_public_data().increase_version();
        lock_w(self.local_repo.as_ref())?.set(profile).wait()?;
        self.save_vault()
    }

    fn claim_schemas(&self) -> Fallible<Rc<dyn ClaimSchemas>> {
        let p = &self.schema_path;
        if !p.exists() {
            std::fs::create_dir_all(p)?;
            ClaimSchemaRegistry::populate_folder(p)?;
        }
        let registry = ClaimSchemaRegistry::import_folder(p)?;
        Ok(Rc::new(registry))
    }

    fn claims(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<Claim>> {
        let profile = self.selected_profile(my_profile_id)?;
        Ok(profile.claims())
    }

    fn add_claim(&mut self, my_profile_id: Option<ProfileId>, claim: Claim) -> Fallible<()> {
        let claim_id = claim.id();
        let mut profile = self.selected_profile(my_profile_id)?;

        // TODO check if schema_id is valid and related schema contents are available
        // TODO validate contents against schema details
        let present_claims = profile.claims();
        let conflicts = present_claims.iter().filter(|old_claim| old_claim.id() == claim_id);
        if conflicts.count() != 0 {
            bail!("Claim {} is already present", claim_id);
        }

        profile.mut_claims().push(claim);
        // TODO this is not public data, should not affect public version
        // profile.mut_public_data().increase_version();
        lock_w(self.local_repo.as_ref())?.set(profile).wait()?;
        debug!("Added claim: {:?}", claim_id);
        self.save_vault()
    }

    fn remove_claim(&mut self, my_profile_id: Option<ProfileId>, id: ClaimId) -> Fallible<()> {
        let mut profile = self.selected_profile(my_profile_id)?;
        let claims = profile.mut_claims();

        let claims_len_before = claims.len();
        claims.retain(|claim| claim.id() != id);
        if claims.len() + 1 != claims_len_before {
            bail!("Claim {} not found", id);
        }

        lock_w(self.local_repo.as_ref())?.set(profile).wait()?;
        debug!("Removed claim: {:?}", id);
        self.save_vault()
    }

    fn sign_claim(
        &self,
        my_profile_id: Option<ProfileId>,
        claim: &SignableClaimPart,
    ) -> Fallible<ClaimProof> {
        let profile = self.selected_profile(my_profile_id)?;
        let claim_bin = serde_json::to_vec(claim)?;
        let signed_message = self.vault()?.sign(&profile.id(), &claim_bin)?;
        let now = TimeStamp::now();
        // TODO make expiration configurable, e.g. request could contain suggested expiration
        let valid_until = now + Duration::from_secs(366 * 24 * 60 * 60);
        Ok(ClaimProof::new(profile.id(), signed_message, now, valid_until))
    }

    fn add_claim_proof(
        &mut self,
        my_profile_id: Option<ProfileId>,
        claim_id: &ClaimId,
        proof: ClaimProof,
    ) -> Fallible<()> {
        let mut profile = self.selected_profile(my_profile_id)?;
        let claim = profile
            .mut_claim(claim_id)
            .ok_or_else(|| format_err!("Claim {} not found", claim_id))?;
        claim.add_proof(proof);
        lock_w(self.local_repo.as_ref())?.set(profile).wait()?;
        debug!("Added proof to claim: {:?}", claim_id);
        self.save_vault()
    }

    fn license_claim(
        &mut self,
        _my_profile_id: Option<ProfileId>,
        _claim: ClaimId,
        // TODO audience, purpose, expiry, etc
    ) -> Fallible<ClaimLicense> {
        unimplemented!()
    }

    fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<Link> {
        let mut profile = self.selected_profile(my_profile_id)?;
        let link = profile.mut_public_data().create_link(peer_profile_id);
        profile.mut_public_data().increase_version();
        lock_w(self.local_repo.as_ref())?.set(profile).wait()?;
        debug!("Created link: {:?}", link);
        self.save_vault()?;
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
        lock_w(self.local_repo.as_ref())?.set(profile).wait()?;
        self.save_vault()
    }

    fn did_homes(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<DidHomeStatus>> {
        let profile = self.selected_profile(my_profile_id)?;
        let hosted_facet = profile.public_data().to_hosted().ok_or_else(|| {
            format_err!("Profile {} lacks hosting details (like homes) filled", profile.id())
        })?;

        Ok(hosted_facet
            .homes
            .iter()
            .filter_map(|rel| {
                let home_id_opt = rel.peer_id(&profile.id()).ok();
                if let None = home_id_opt {
                    warn!("BUG: found relation of two other peers in profile as hosting proof")
                }
                home_id_opt
            })
            // TODO online status is not gathered yet
            .map(|home_did| DidHomeStatus { home_did: home_did.to_string(), online: false })
            .collect())
    }

    // TODO functions that communicate on the network must be async, and probably implemented on a different trait
    fn register_home(
        &mut self,
        my_id: Option<ProfileId>,
        home_id: &ProfileId,
        addr_hints: &[Multiaddr],
        network: &NetworkState,
    ) -> AsyncFallible<()> {
        let init_fn = || {
            let profile = self.selected_profile(my_id)?;
            ensure!(
                !profile.public_data().is_hosted_on(home_id),
                "Profile {} is already registered to home {}",
                profile.id(),
                home_id
            );
            let signer = self.vault()?.signer(&profile.id())?;
            let crawler = network.home_node_crawler.clone();
            let network = network.home_connector.to_owned();
            let host_half = RelationHalfProof::new(
                RelationProof::RELATION_TYPE_HOSTED_ON_HOME,
                home_id,
                signer.as_ref(),
            )?;
            Ok((profile, signer, network, crawler, host_half))
        };

        let (mut profile, signer, network, crawler, host_half_proof) = match init_fn() {
            Ok(v) => v,
            Err(e) => return Box::new(Err(e).into_future()),
        };

        let local_repo = self.local_repo.clone();
        let home_id_copy = home_id.to_owned();
        let fut = network
            .connect(home_id, addr_hints, signer)
            // TODO report home node to crawler
            .and_then(move |home| { home.fetch(&home_id_copy).map(|prof| (home, prof))})
            .and_then(move|(home, prof)| {
                let mut crawler_lock = crawler.try_write()
                    .map_err(|e| format_err!("Failed to lock crawler: {}", e))?;
                crawler_lock.add(&prof).map(|()| home)
            } )
            .and_then(move |home| home.register(host_half_proof).map_err(|e| e.into()))
            .and_then(move |host_proof| {
                profile.mut_public_data().add_hosted_on(&host_proof)?;
                profile.mut_public_data().increase_version();
                Ok(profile)
            } )
            .and_then(move |profile|
                match lock_w(local_repo.as_ref()) {
                    Ok(mut repo) => {
                        Either::A(repo.set(profile))
                        // TODO publish profile data with new home to remote distributed repository
                    }
                    Err(e) => Either::B(Err(e).into_future()),
                }
            );

        Box::new(fut)
    }
}
