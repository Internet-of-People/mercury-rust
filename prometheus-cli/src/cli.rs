use std::path::PathBuf;

use failure::{bail, ensure, err_msg, Fallible};
use log::*;

use crate::options::*;
use osg::model::*;
use osg::repo::ProfileRepository;
use osg::vault::*;

pub struct CommandContext {
    vault_path: PathBuf,
    vault: Option<Box<ProfileVault>>,
    local_repo: Box<ProfileRepository>, // TODO should this be LocalProfileRepository instead?
    remote_repo: Box<ProfileRepository>,
}

impl CommandContext {
    pub fn new(
        vault_path: PathBuf,
        vault: Option<Box<ProfileVault>>,
        local_repo: Box<ProfileRepository>,
        remote_repo: Box<ProfileRepository>,
    ) -> Self {
        Self {
            vault_path,
            vault,
            local_repo,
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
        let new_vault = HdProfileVault::create(seed);
        self.vault.replace(Box::new(new_vault));
        Ok(())
    }

    fn restore_all_profiles(&mut self) -> Fallible<()> {
        let mut all_profile_ids = self.vault().list()?;
        all_profile_ids.append(&mut self.vault().list_gap()?);

        let mut restore_count = 0;
        for profile_id in all_profile_ids.iter() {
            let profile_res = self.remote_repo.get(&profile_id);
            if let Err(e) = profile_res {
                info!("  Remote profile {} not found: {}", profile_id, e);
                continue;
            }
            self.local_repo
                .set(profile_id.clone(), profile_res.unwrap())?;
            self.mut_vault().restore_id(&profile_id)?;
            restore_count += 1;
            info!("  Successfully restored profile {}", profile_id);
        }

        info!(
            "Tried {} profiles, successfully restored {}",
            all_profile_ids.len(),
            restore_count
        );
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

    pub fn execute(&mut self, command: Command) -> Fallible<()> {
        match command {
            Command::Create(CreateCommand::Link {
                my_profile_id,
                peer_profile_id,
            }) => {
                let mut profile = self.selected_profile(my_profile_id)?;
                let link = profile.create_link(&peer_profile_id);
                self.local_repo.set(profile.id().to_owned(), profile)?;
                debug!("Created link: {:?}", link);
                info!("Created link to peer profile {}", peer_profile_id);
            }

            Command::Create(CreateCommand::Profile) => {
                let new_profile_id = self.mut_vault().create_id()?;
                let empty_profile = ProfileData::empty(&new_profile_id);
                self.local_repo
                    .set(new_profile_id.to_owned(), empty_profile)?;
                info!("Created and activated profile with id {}", new_profile_id);
            }

            Command::Clear(ClearCommand::Attribute { my_profile_id, key }) => {
                let mut profile = self.selected_profile(my_profile_id)?;
                profile.clear_attribute(&key);
                self.local_repo.set(profile.id().to_owned(), profile)?;
                info!("Cleared attribute: {}", key);
            }

            Command::List(ListCommand::IncomingLinks { my_profile_id }) => {
                let profile = self.selected_profile(my_profile_id)?;
                let followers = self.remote_repo.followers(profile.id())?;
                info!("You have {} followers", followers.len());
                for (idx, follower) in followers.iter().enumerate() {
                    info!("  {}: {:?}", idx, follower);
                }
            }

            Command::List(ListCommand::Profiles) => {
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
            }

            Command::Remove(RemoveCommand::Link {
                my_profile_id,
                peer_profile_id,
            }) => {
                let mut profile = self.selected_profile(my_profile_id)?;
                profile.remove_link(&peer_profile_id);
                self.local_repo.set(profile.id().to_owned(), profile)?;
                info!("Removed link from profile {}", peer_profile_id);
            }

            Command::Set(SetCommand::ActiveProfile { my_profile_id }) => {
                self.mut_vault().set_active(&my_profile_id)?;
                info!("Active profile was set to {}", my_profile_id);
            }

            Command::Set(SetCommand::Attribute {
                my_profile_id,
                key,
                value,
            }) => {
                let mut profile = self.selected_profile(my_profile_id)?;
                info!("Setting attribute {} to {}", key, value);
                profile.set_attribute(key, value);
                self.local_repo.set(profile.id().to_owned(), profile)?;
            }

            Command::Show(ShowCommand::Profile { profile_id, local }) => {
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
            }

            Command::Generate(GenerateCommand::Vault) => {
                generate_vault();
            }

            Command::Restore(RestoreCommand::Vault { demo }) => {
                self.restore_vault(demo)?;
                self.restore_all_profiles()?;
            }

            Command::Restore(RestoreCommand::Profiles {}) => {
                self.restore_all_profiles()?;
            }

            Command::Restore(RestoreCommand::Profile { my_profile_id }) => {
                let profile_id = self.selected_profile_id(my_profile_id)?;
                self.mut_vault().restore_id(&profile_id)?;
                debug!("Fetching profile {} from remote repository", profile_id);
                let profile = self.remote_repo.get(&profile_id)?;
                self.local_repo.set(profile_id.clone(), profile)?;
                info!("Restored profile {} from remote repository", profile_id);
            }

            Command::Publish(PublishCommand::Profile { my_profile_id }) => {
                let profile = self.selected_profile(my_profile_id)?;
                info!("Publishing profile {} to remote repository", profile.id());
                self.remote_repo.set(profile.id().to_owned(), profile)?;
            }
        };

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
