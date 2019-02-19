use failure::{ensure, err_msg, Fallible};
use log::*;
use structopt::StructOpt;

use morpheus_storage::*;
use prometheus::vault::*;

pub struct CommandContext {
    vault: Option<Box<ProfileVault>>,
    store: Box<ProfileStore>,
}

impl CommandContext {
    pub fn new(vault: Option<Box<ProfileVault>>, store: Box<ProfileStore>) -> Self {
        Self { vault, store }
    }

    /// # Panic
    /// If there is no vault given to `new`
    pub fn vault(&self) -> &ProfileVault {
        self.vault.as_ref().unwrap().as_ref()
    }

    /// # Panic
    /// If there is no vault given to `new`
    pub fn mut_vault(&mut self) -> &mut ProfileVault {
        self.vault.as_mut().unwrap().as_mut()
    }

    pub fn take_vault(&mut self) -> Option<Box<ProfileVault>> {
        self.vault.take()
    }

    pub fn replace_vault(&mut self, new_vault: Box<ProfileVault>) -> Option<Box<ProfileVault>> {
        self.vault.replace(new_vault)
    }

    pub fn store(&self) -> &ProfileStore {
        self.store.as_ref()
    }

    pub fn mut_store(&mut self) -> &mut ProfileStore {
        self.store.as_mut()
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "prometheus", about = "Command line interface for Prometheus")]
pub enum Command {
    #[structopt(name = "status")]
    Status,

    #[structopt(name = "list")]
    /// List profiles or followers
    List(ListCommand),

    /// Show profile details
    #[structopt(name = "show")]
    Show(ShowCommand),

    #[structopt(name = "create")]
    /// Create profile or link
    Create(CreateCommand),

    #[structopt(name = "remove")]
    /// Remove link // TODO (or profile?)
    Remove(RemoveCommand),

    #[structopt(name = "set")]
    /// Set active profile or attribute
    Set(SetCommand),

    #[structopt(name = "clear")]
    /// Clear attribute
    Clear(ClearCommand),

    #[structopt(name = "vault")]
    Vault(VaultCommand),
}

fn selected_profile(
    ctx: &CommandContext,
    my_profile_option: Option<ProfileId>,
) -> Fallible<ProfilePtr> {
    let profile_opt = my_profile_option
        // TODO ideally this should be or_else, but mixing Option<T> with Result<Option<T>,E> is complex
        .or(ctx.vault().get_active()?)
        .and_then(|profile_id| ctx.store().get(&profile_id));
    ensure!(
        profile_opt.is_some(),
        "Command option my_profile_id is unspecified and no active default profile was found"
    );
    Ok(profile_opt.unwrap())
}

impl Command {
    pub fn needs_vault(&self) -> bool {
        match self {
            Command::Vault(_) => false,
            _ => true,
        }
    }

    pub fn execute(self, ctx: &mut CommandContext) -> Fallible<()> {
        match self {
            Command::Create(CreateCommand::Link {
                my_profile_id,
                peer_profile_id,
            }) => {
                let profile = selected_profile(ctx, my_profile_id)?;
                let link = profile.borrow_mut().create_link(&peer_profile_id)?;
                info!("Created link to profile {:?}", link);
            }

            Command::Create(CreateCommand::Profile) => {
                let new_profile_id = ctx.mut_vault().create_id()?;
                let created_profile_ptr = ctx.mut_store().create(&new_profile_id)?;
                let created_profile = created_profile_ptr.borrow();
                info!(
                    "Created and activated profile with id {}",
                    created_profile.id()
                );
            }

            Command::Clear(ClearCommand::Attribute { my_profile_id, key }) => {
                let profile = selected_profile(ctx, my_profile_id)?;
                info!("Clearing attribute: {}", key);
                profile.borrow_mut().clear_attribute(&key)?;
            }

            Command::List(ListCommand::IncomingLinks { my_profile_id }) => {
                let profile = selected_profile(ctx, my_profile_id)?;
                let followers = profile.borrow().followers()?;
                info!("Received {} followers", followers.len());
                for (idx, follower) in followers.iter().enumerate() {
                    info!("  {}: {:?}", idx, follower);
                }
            }

            Command::List(ListCommand::Profiles) => {
                let profile_ids = ctx.vault().list()?;
                info!("Has {} profiles", profile_ids.len());
                for (i, profile_id) in profile_ids.iter().enumerate() {
                    // TODO mark active profile somehow
                    info!("  {}: {}", i, profile_id);
                }
            }

            Command::Remove(RemoveCommand::Link {
                my_profile_id,
                peer_profile_id,
            }) => {
                let profile = selected_profile(ctx, my_profile_id)?;
                profile.borrow_mut().remove_link(&peer_profile_id)?;
                info!("Removed link from profile {}", peer_profile_id);
            }

            Command::Set(SetCommand::ActiveProfile { my_profile_id }) => {
                ctx.mut_vault().set_active(&my_profile_id)?;
                info!("Active profile was set to {}", my_profile_id);
            }

            Command::Set(SetCommand::Attribute {
                my_profile_id,
                key,
                value,
            }) => {
                let profile = selected_profile(ctx, my_profile_id)?;
                info!("Setting attribute {} to {}", key, value);
                profile.borrow_mut().set_attribute(&key, &value)?;
            }

            Command::Show(ShowCommand::Profile { profile_id }) => {
                // NOTE must also work with a profile that is not ours
                let profile_ptr_opt = ctx.store().get(&profile_id);
                let profile_ptr =
                    profile_ptr_opt.ok_or_else(|| err_msg("Failed to retrieve profile"))?;
                let links = profile_ptr.borrow().links()?;
                let metadata = profile_ptr.borrow().metadata()?;

                info!("Details of profile id {}", profile_id);
                info!("  {} attributes:", metadata.len());
                for (i, attribute) in metadata.iter().enumerate() {
                    info!("    {}: {:?}", i, attribute);
                }
                info!("  {} subscriptions:", links.len());
                for (i, peer_id) in links.iter().enumerate() {
                    info!("    {}: {:?}", i, peer_id);
                }
            }

            Command::Status => {
                let active_profile_opt = ctx.vault().get_active()?;
                match active_profile_opt {
                    Some(active_prof) => info!("Your active profile is {}", active_prof),
                    None => info!("You still don't have an active profile set"),
                };
                // TODO what status to display besides active (default) profile?
            }

            Command::Vault(VaultCommand::Generate) => {
                VaultCommand::generate();
            }

            Command::Vault(VaultCommand::Restore { demo }) => {
                VaultCommand::restore(ctx, demo);
            }
        };

        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum ListCommand {
    #[structopt(name = "profiles")]
    /// List profiles
    Profiles,

    #[structopt(name = "followers")]
    /// List followers
    IncomingLinks {
        #[structopt(long = "my_profile_id")]
        /// List public followers of this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,
    },
}

#[derive(Debug, StructOpt)]
pub enum ShowCommand {
    #[structopt(name = "profile")]
    /// Show profile
    Profile {
        #[structopt(long = "profile_id")]
        /// Profile id to be shown, either yours or remote
        profile_id: ProfileId,
    },
}

#[derive(Debug, StructOpt)]
pub enum CreateCommand {
    #[structopt(name = "profile")]
    /// Create profile
    Profile, // TODO how to specify to keep current or new profile should be active/default

    #[structopt(name = "link")]
    /// Create link, i.e. follow/subscribe to a remote profile
    Link {
        #[structopt(long = "my_profile_id")]
        /// Add link to this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt(long = "peer_profile_id")]
        /// Create link to this remote profile
        peer_profile_id: ProfileId,
        // TODO is an optional "relation_type" needed here?
    },
}

#[derive(Debug, StructOpt)]
pub enum RemoveCommand {
    #[structopt(name = "link")]
    /// Remove link, i.e. unfollow/unsubscribe from another profile
    Link {
        #[structopt(long = "my_profile_id")]
        /// Remove link from this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt(long = "peer_profile_id")]
        /// Remove link with this remote profile
        peer_profile_id: ProfileId,
    },
}

#[derive(Debug, StructOpt)]
pub enum SetCommand {
    #[structopt(name = "active-profile")]
    /// Show profile
    ActiveProfile {
        // TODO is activation by profile NUMBER needed or is this enough?
        #[structopt(long = "my_profile_id")]
        /// Profile id to be activated
        my_profile_id: ProfileId,
    },

    #[structopt(name = "attribute")]
    /// Set attribute with name to specified value
    Attribute {
        #[structopt(long = "my_profile_id")]
        /// Set attribute to this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt(long = "key")]
        /// Attribute name
        key: AttributeId,

        #[structopt(long = "value")]
        /// Attribute value
        value: AttributeValue,
    },
}

#[derive(Debug, StructOpt)]
pub enum ClearCommand {
    #[structopt(name = "attribute")]
    /// Clear attribute
    Attribute {
        #[structopt(long = "my_profile_id")]
        /// Clear attribute from this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt(long = "key")]
        /// Attribute name
        key: AttributeId,
    },
}

#[derive(Debug, StructOpt)]
pub enum VaultCommand {
    #[structopt(name = "generate")]
    Generate,

    #[structopt(name = "restore")]
    Restore {
        #[structopt(long = "demo")]
        demo: bool,
    },
}

impl VaultCommand {
    pub fn generate() {
        let new_bip39_phrase = morpheus_keyvault::Seed::generate_bip39();
        let words = new_bip39_phrase.split(' ');
        info!(
            r#"Make sure you back these words up somewhere safe
and rerun the application with the restore parameter!"#
        );
        words
            .enumerate()
            .for_each(|(i, word)| info!("    {:2}: {}", i + 1, word));
    }

    pub fn restore(ctx: &mut CommandContext, demo: bool) {
        let seed = morpheus_keyvault::Seed::from_bip39("include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already").unwrap();
        let vault = DummyProfileVault::create(seed);
        ctx.replace_vault(Box::new(vault));
    }
}
