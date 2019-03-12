use std::net::SocketAddr;
use std::path::PathBuf;

use failure::{bail, ensure, err_msg, Fallible};
use log::*;
use structopt::StructOpt;

use osg::model::*;
use osg::repo::ProfileRepository;
use osg::vault::*;

pub struct CommandContext {
    vault_path: PathBuf,
    vault: Option<Box<ProfileVault>>,
    repo: Box<ProfileRepository>,
}

impl CommandContext {
    pub fn new(
        vault_path: PathBuf,
        vault: Option<Box<ProfileVault>>,
        repo: Box<ProfileRepository>,
    ) -> Self {
        Self {
            vault_path,
            vault,
            repo,
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

    pub fn repo(&self) -> &ProfileRepository {
        self.repo.as_ref()
    }

    pub fn mut_repo(&mut self) -> &mut ProfileRepository {
        self.repo.as_mut()
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "prometheus",
    about = "Command line interface for Prometheus",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
pub struct Options {
    #[structopt(long = "keyvault", raw(value_name = r#""FILE""#), parse(from_os_str))]
    /// Path of the keyvault file. Default: OS-specific app_cfg_dir/prometheus/vault.dat
    pub keyvault_path: Option<PathBuf>,

    #[structopt(long = "profiles", raw(value_name = r#""FILE""#), parse(from_os_str))]
    /// Path of the keyvault file. Default: OS-specific app_cfg_dir/prometheus/profiles.dat
    pub profile_repo_path: Option<PathBuf>,

    #[structopt(
        long = "storage",
        default_value = "127.0.0.1:6161",
        raw(value_name = r#""IP:PORT""#)
    )]
    /// IPv4/6 address of the storage backend.
    pub storage_address: SocketAddr,

    #[structopt(long = "timeout", default_value = "10", raw(value_name = r#""SECS""#))]
    /// Number of seconds used for network timeouts
    pub network_timeout_secs: u64,

    #[structopt(
        long = "logger_config",
        default_value = "log4rs.yml",
        raw(value_name = r#""FILE""#),
        parse(from_os_str)
    )]
    /// Config file for log4rs (YAML).
    pub logger_config: PathBuf,

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

    #[structopt(name = "list")]
    /// List profiles or followers
    List(ListCommand),

    /// Show profile details
    #[structopt(name = "show")]
    Show(ShowCommand),

    #[structopt(name = "create")]
    /// Create profile or link
    Create(CreateCommand),

    // TODO consider if removing profile is needed?
    #[structopt(name = "remove")]
    /// Remove link
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
) -> Fallible<ProfileData> {
    let profile_id_opt = my_profile_option.or_else(|| ctx.vault().get_active().ok()?);
    let profile_id = match profile_id_opt {
        Some(profile_id) => profile_id,
        None => bail!(
            "Command option my_profile_id is unspecified and no active default profile was found"
        ),
    };
    info!("Your active profile is {}", profile_id);
    let profile = ctx.repo().get(&profile_id)?;
    Ok(profile)
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
                let mut profile = selected_profile(ctx, my_profile_id)?;
                let link = profile.create_link(&peer_profile_id);
                ctx.mut_repo().set(profile.id().to_owned(), profile)?;
                debug!("Created link: {:?}", link);
                info!("Created link to peer profile {}", peer_profile_id);
            }

            Command::Create(CreateCommand::Profile) => {
                let new_profile_id = ctx.mut_vault().create_id()?;
                let empty_profile = ProfileData::empty(&new_profile_id);
                ctx.mut_repo()
                    .set(new_profile_id.to_owned(), empty_profile)?;
                info!("Created and activated profile with id {}", new_profile_id);
            }

            Command::Clear(ClearCommand::Attribute { my_profile_id, key }) => {
                let mut profile = selected_profile(ctx, my_profile_id)?;
                profile.clear_attribute(&key);
                ctx.mut_repo().set(profile.id().to_owned(), profile)?;
                info!("Cleared attribute: {}", key);
            }

            Command::List(ListCommand::IncomingLinks { my_profile_id }) => {
                let profile = selected_profile(ctx, my_profile_id)?;
                let followers = ctx.repo.followers(profile.id())?;
                info!("You have {} followers", followers.len());
                for (idx, follower) in followers.iter().enumerate() {
                    info!("  {}: {:?}", idx, follower);
                }
            }

            Command::List(ListCommand::Profiles) => {
                let profile_ids = ctx.vault().list()?;
                info!("You have {} profiles", profile_ids.len());
                let active_profile_opt = ctx.vault().get_active()?;
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
                let mut profile = selected_profile(ctx, my_profile_id)?;
                profile.remove_link(&peer_profile_id);
                ctx.mut_repo().set(profile.id().to_owned(), profile)?;
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
                let mut profile = selected_profile(ctx, my_profile_id)?;
                info!("Setting attribute {} to {}", key, value);
                profile.set_attribute(key, value);
                ctx.mut_repo().set(profile.id().to_owned(), profile)?;
            }

            Command::Show(ShowCommand::Profile { profile_id }) => {
                // NOTE must also work with a profile that is not ours
                let profile = ctx.repo().get(&profile_id)?;
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
        #[structopt()] // long = "my_profile_id"
        /// List public followers of this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,
    },
}

#[derive(Debug, StructOpt)]
pub enum ShowCommand {
    #[structopt(name = "profile")]
    /// Show profile
    Profile {
        #[structopt()] // long = "profile_id"
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

        #[structopt()] // long = "peer_profile_id"
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

        #[structopt()] // long = "peer_profile_id"
        /// Remove link with this remote profile
        peer_profile_id: ProfileId,
    },
}

#[derive(Debug, StructOpt)]
pub enum SetCommand {
    #[structopt(name = "active_profile")]
    /// Show profile
    ActiveProfile {
        // TODO is activation by profile NUMBER needed or is this enough?
        //      If enough, should be a mandatory positional parameter instead of a named one.
        #[structopt()] // long = "my_profile_id"
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

        #[structopt()] // long = "key"
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
    ctx.replace_vault(Box::new(new_vault));
    Ok(())
}
