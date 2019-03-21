use std::path::PathBuf;

use failure::{bail, ensure, err_msg, Fallible};
use log::*;

use crate::model::*;
use crate::repo::ProfileRepository;
use crate::vault::{self, ProfileVault};

pub type ApiRes = Fallible<()>;
pub trait Api {
    fn restore_vault(&mut self, demo: bool) -> ApiRes;
    fn restore_all_profiles(&mut self) -> ApiRes;
    fn list_profiles(&mut self) -> ApiRes;
    fn set_active_profile(&mut self, my_profile_id: ProfileId) -> ApiRes;

    fn create_profile(&mut self) -> ApiRes;
    fn restore_profile(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes;
    fn publish_profile(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes;
    fn show_profile(&mut self, profile_id: Option<ProfileId>, local: bool) -> ApiRes;

    fn list_incoming_links(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes;

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
    local_repo: Box<ProfileRepository>,
    base_repo: Box<ProfileRepository>,
    remote_repo: Box<ProfileRepository>,
}

impl Context {
    pub fn new(
        vault_path: PathBuf,
        vault: Option<Box<ProfileVault>>,
        local_repo: Box<ProfileRepository>,
        base_repo: Box<ProfileRepository>,
        remote_repo: Box<ProfileRepository>,
    ) -> Self {
        Self {
            vault_path,
            vault,
            local_repo,
            base_repo,
            remote_repo,
        }
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

    fn restore_vault(&mut self, demo: bool) -> Fallible<()> {
        let old_vault_op = self.take_vault();
        ensure!(
            old_vault_op.is_none(),
            r#"You already have an active vault.
Please delete {}
before trying to restore another vault."#,
            self.vault_path.to_string_lossy()
        );

        let phrase = if demo {
            "include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already".to_owned()
        } else {
            read_phrase()?
        };

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

    fn selected_profile(&self, my_profile_option: Option<ProfileId>) -> Fallible<ProfileData> {
        let profile_id = self.selected_profile_id(my_profile_option)?;
        let profile = self.local_repo.get(&profile_id)?;
        Ok(profile)
    }

    fn restore_one_profile(&mut self, profile_id: &ProfileId) -> ApiRes {
        let profile = self.remote_repo.get(profile_id)?;
        self.local_repo.set(profile_id.clone(), profile.clone())?;
        self.base_repo.set(profile_id.clone(), profile)?;
        self.mut_vault().restore_id(&profile_id)?;
        info!("  Successfully restored profile {}", profile_id);
        Ok(())
    }
}

impl Api for Context {
    fn list_profiles(&mut self) -> ApiRes {
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

    fn list_incoming_links(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes {
        let profile = self.selected_profile(my_profile_id)?;
        let followers = self.remote_repo.followers(profile.id())?;
        info!("You have {} followers", followers.len());
        for (idx, follower) in followers.iter().enumerate() {
            info!("  {}: {:?}", idx, follower);
        }
        Ok(())
    }

    fn show_profile(&mut self, profile_id: Option<ProfileId>, local: bool) -> ApiRes {
        // NOTE must also work with a profile that is not ours
        let profile_id = self.selected_profile_id(profile_id)?;
        let repo = if local {
            &self.local_repo
        } else {
            &self.remote_repo
        };
        let profile = repo.get(&profile_id)?;
        let links = profile.links();
        let attributes = profile.attributes();

        info!("Details of profile id {}", profile_id);
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
        let new_profile_id = self.mut_vault().create_id()?;
        let empty_profile = ProfileData::empty(&new_profile_id);
        self.local_repo
            .set(new_profile_id.to_owned(), empty_profile)?;
        info!("Created and activated profile with id {}", new_profile_id);
        Ok(())
    }

    fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: ProfileId,
    ) -> ApiRes {
        let mut profile = self.selected_profile(my_profile_id)?;
        let link = profile.create_link(&peer_profile_id);
        self.local_repo.set(profile.id().to_owned(), profile)?;
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
        profile.remove_link(&peer_profile_id);
        self.local_repo.set(profile.id().to_owned(), profile)?;
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
        profile.set_attribute(key, value);
        self.local_repo.set(profile.id().to_owned(), profile)
    }

    fn clear_attribute(&mut self, my_profile_id: Option<ProfileId>, key: AttributeId) -> ApiRes {
        let mut profile = self.selected_profile(my_profile_id)?;
        profile.clear_attribute(&key);
        self.local_repo.set(profile.id().to_owned(), profile)?;
        info!("Cleared attribute: {}", key);
        Ok(())
    }

    fn restore_vault(&mut self, demo: bool) -> ApiRes {
        self.restore_vault(demo)?;
        self.restore_all_profiles()
    }

    fn restore_profile(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes {
        let profile_id = self.selected_profile_id(my_profile_id)?;
        self.mut_vault().restore_id(&profile_id)?;
        debug!("Fetching profile {} from remote repository", profile_id);
        let profile = self.remote_repo.get(&profile_id)?;
        self.local_repo.set(profile_id.clone(), profile)?;
        info!("Restored profile {} from remote repository", profile_id);
        Ok(())
    }

    fn restore_all_profiles(&mut self) -> ApiRes {
        let profiles = self.vault().profiles()?;
        let len = self.vault().len();

        let mut try_count = 0;
        let mut restore_count = 0;
        for idx in 0..len {
            try_count += 1;
            let profile_id = profiles.id(idx as i32)?;
            if let Err(e) = self.restore_one_profile(&profile_id) {
                info!(
                    "  Profile {} not found on this repository: {}",
                    profile_id, e
                );
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
            if let Err(e) = self.restore_one_profile(&profile_id) {
                debug!("  Profile {} was tried, but not found: {}", profile_id, e);
                idx += 1;
                continue;
            }
            end = idx + vault::GAP;
            idx += 1;
            restore_count += 1;
        }

        info!(
            "Tried {} profiles, successfully restored {}",
            try_count, restore_count
        );
        Ok(())
    }

    fn publish_profile(&mut self, my_profile_id: Option<ProfileId>) -> ApiRes {
        let profile = self.selected_profile(my_profile_id)?;
        info!("Publishing profile {} to remote repository", profile.id());
        self.remote_repo
            .set(profile.id().to_owned(), profile.clone())?;
        self.base_repo.set(profile.id().to_owned(), profile)
    }
}

pub fn generate_vault() {
    let new_bip39_phrase = keyvault::Seed::generate_bip39();
    let words = new_bip39_phrase.split(' ');
    warn!(
        r#"Make sure you back these words up somewhere safe
and run the 'restore vault' command of this application first!"#
    );
    words
        .enumerate()
        .for_each(|(i, word)| info!("    {:2}: {}", i + 1, word));
}

fn read_phrase() -> Fallible<String> {
    use std::io::BufRead;
    use std::io::Write;

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();
    stdout_lock.write_fmt(format_args!(
        "Please type the words you backed up one-by-one pressing enter after each:\n"
    ))?;

    let mut words = Vec::with_capacity(24);
    for i in 1..=24 {
        loop {
            let mut buffer = String::with_capacity(10);
            stdout_lock.write_fmt(format_args!("  {:2}> ", i))?; // no newline at the end for this prompt!
            stdout_lock.flush()?; // without this, nothing is written on the console
            stdin_lock.read_line(&mut buffer)?;
            buffer = buffer.trim().to_owned();
            if keyvault::Seed::check_word(&buffer) {
                words.push(buffer);
                break;
            } else {
                stdout_lock.write_fmt(format_args!(
                    "{} is not in the dictionary, please retry entering it\n",
                    buffer
                ))?;
            }
        }
    }
    let phrase = words.join(" ");

    debug!("You entered: {}", phrase);

    Ok(phrase)
}
