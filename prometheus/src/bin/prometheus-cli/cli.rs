use std::net::SocketAddr;
use std::path::PathBuf;

use failure::{ensure, err_msg, Fallible};
use log::*;
use structopt::StructOpt;

use morpheus_storage::*;
use prometheus::vault::*;

pub struct CommandContext {
    vault_path: PathBuf,
    vault: Option<Box<ProfileVault>>,
    store: Box<ProfileStore>,
}

impl CommandContext {
    pub fn new(
        vault_path: PathBuf,
        vault: Option<Box<ProfileVault>>,
        store: Box<ProfileStore>,
    ) -> Self {
        Self {
            vault_path,
            vault,
            store,
        }
    }

    // TODO there should be no version of vault getters that panic
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
#[structopt(
    name = "prometheus",
    about = "Command line interface for Prometheus",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
pub struct Options {
    #[structopt(
        long = "storage",
        default_value = "127.0.0.1:6161",
        raw(value_name = r#""ADDRESS""#)
    )]
    /// IPv4/6 address of the storage backend used for this demo
    pub storage_address: SocketAddr,

    #[structopt(long = "timeout", default_value = "10", raw(value_name = r#""SECS""#))]
    /// Number of seconds used for network timeouts
    pub network_timeout_secs: u64,

    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "generate")]
    /// Generate a phraselist needed to create a profile vault
    Generate(GenerateCommand),

    #[structopt(name = "restore")]
    /// Restore profile vault from a phraselist
    Restore(RestoreCommand),

    #[structopt(name = "status")]
    /// Show the status of your profile vault
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
}

fn selected_profile(
    ctx: &CommandContext,
    my_profile_option: Option<ProfileId>,
) -> Fallible<ProfilePtr> {
    let profile_opt = my_profile_option
        .or_else(|| ctx.vault().get_active().ok()?)
        .and_then(|profile_id| {
            info!("Your active profile is {}", profile_id);
            ctx.store().get(&profile_id)
        });
    ensure!(
        profile_opt.is_some(),
        "Command option my_profile_id is unspecified and no active default profile was found"
    );
    Ok(profile_opt.unwrap())
}

impl Command {
    pub fn needs_vault(&self) -> bool {
        match self {
            Command::Generate(_) | Command::Restore(_) => false,
            Command::Show(ShowCommand::Profile { .. }) => false,
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
                debug!("Created link: {:?}", link);
                info!("Created link to peer profile {}", peer_profile_id);
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
                info!("You have {} followers", followers.len());
                for (idx, follower) in followers.iter().enumerate() {
                    info!("  {}: {:?}", idx, follower);
                }
            }

            Command::List(ListCommand::Profiles) => {
                let profile_ids = ctx.vault().list()?;
                info!("You have {} profiles", profile_ids.len());
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

            Command::Generate(GenerateCommand::Vault) => {
                generate_vault();
            }

            Command::Restore(RestoreCommand::Vault { demo }) => {
                restore_vault(ctx, demo)?;
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
        //      If enough, should be a mandatory positional parameter instead of a named one.
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
pub enum GenerateCommand {
    #[structopt(name = "vault")]
    /// Generate a phraselist needed to create a profile vault
    Vault,
}

#[derive(Debug, StructOpt)]
pub enum RestoreCommand {
    #[structopt(name = "vault")]
    /// (Re)build a profile vault (needed for most commands) from a phraselist
    Vault {
        #[structopt(long = "demo")]
        demo: bool,
    },
}

pub fn generate_vault() {
    let new_bip39_phrase = morpheus_keyvault::Seed::generate_bip39();
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
            if morpheus_keyvault::Seed::check_word(&buffer) {
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

fn restore_vault(ctx: &mut CommandContext, demo: bool) -> Fallible<()> {
    let old_vault_op = ctx.take_vault();
    ensure!(
        old_vault_op.is_none(),
        r#"You already have an active vault.
Please delete {}
before trying to restore another vault."#,
        ctx.vault_path.to_string_lossy()
    );

    let phrase = if demo {
        "include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already".to_owned()
    } else {
        read_phrase()?
    };

    let seed_res = morpheus_keyvault::Seed::from_bip39(&phrase);
    let seed = match seed_res {
        Ok(seed) => Ok(seed),
        Err(e) => {
            if let Some(morpheus_keyvault::Bip39ErrorKind::InvalidChecksum) =
                e.find_root_cause().downcast_ref()
            {
                Err(err_msg("All the words entered were valid, still the checksum was wrong.\nIs the order of the words correct?"))
            } else {
                Err(e)
            }
        }
    }?;
    let new_vault = DummyProfileVault::create(seed);
    ctx.replace_vault(Box::new(new_vault));
    Ok(())
}
