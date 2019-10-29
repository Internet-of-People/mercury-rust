use std::path::PathBuf;

use async_trait::async_trait;
use failure::Fallible;
use log::*;
use structopt::StructOpt;

use crate::seed::{read_phrase, show_generated_phrase};
use claims::model::*;
use prometheus::vault::api::*;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "prometheus-cli",
    about = "Command line interface for Prometheus",
    setting = structopt::clap::AppSettings::ColoredHelp
)]
pub struct Options {
    #[structopt(long = "address", default_value = "127.0.0.1:8080", value_name = "IP:PORT")]
    /// IPv4/6 address of the remote profile repository.
    pub prometheus_address: String,

    // #[structopt(long = "timeout", default_value = "10", value_name = "SECS")]
    // /// Number of seconds used for network timeouts
    // pub network_timeout_secs: u64,
    #[structopt(long, default_value = "log4rs.yml", value_name = "FILE", parse(from_os_str))]
    /// Config file for log4rs (YAML).
    pub logger_config: PathBuf,

    #[structopt(subcommand)]
    pub command: CommandVerb,
}

pub type CmdRes = Fallible<()>;
#[async_trait(?Send)]
pub trait Command {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes;
}

#[derive(Debug, StructOpt)]
pub enum CommandVerb {
    #[structopt(name = "generate")]
    /// Generate a phraselist needed to create a profile vault
    Generate(GenerateCommand),

    #[structopt(name = "restore")]
    /// Restore profile vault from a phraselist or profile from remote repository
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

    #[structopt(name = "publish")]
    /// Publish local profile version to remote profile repository
    Publish(PublishCommand),

    #[structopt(name = "revert")]
    /// Revert unpublished profile to previous version
    Revert(RevertCommand),
}

#[async_trait(?Send)]
impl Command for CommandVerb {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        use CommandVerb::*;
        let sub: Box<dyn Command> = match *self {
            Generate(sub) => Box::new(sub),
            Restore(sub) => Box::new(sub),
            List(sub) => Box::new(sub),
            Show(sub) => Box::new(sub),
            Create(sub) => Box::new(sub),
            Remove(sub) => Box::new(sub),
            Set(sub) => Box::new(sub),
            Clear(sub) => Box::new(sub),
            Publish(sub) => Box::new(sub),
            Revert(sub) => Box::new(sub),
        };
        sub.execute(api).await
    }
}

#[derive(Debug, StructOpt)]
pub enum ListCommand {
    #[structopt(name = "profiles")]
    /// List profiles
    Profiles,
    // #[structopt(name = "followers")]
    // /// List followers
    // IncomingLinks {
    //     #[structopt()]
    //     /// List public followers of this profile of yours if other than the active one
    //     my_profile_id: Option<ProfileId>,
    // },
}

#[async_trait(?Send)]
impl Command for ListCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        use ListCommand::*;
        match *self {
            Profiles => {
                let profiles = api.list_vault_records().await?;
                info!("You have {} profiles", profiles.len());
                let active_profile_opt = api.get_active_profile().await?;
                for profile_record in profiles.iter() {
                    let status = match active_profile_opt {
                        Some(ref active_profile) => {
                            if *active_profile == profile_record.id() {
                                " (active)"
                            } else {
                                ""
                            }
                        }
                        None => "",
                    };
                    info!("  {}: {}{}", profile_record.label(), profile_record.id(), status);
                }
            } // IncomingLinks { my_profile_id } => {
              //     let followers = api.list_incoming_links(my_profile_id)?;
              //     info!("You have {} followers", followers.len());
              //     for (idx, follower) in followers.iter().enumerate() {
              //         info!("  {}: {:?}", idx, follower);
              //     }
              // }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum ShowCommand {
    #[structopt(name = "profile")]
    /// Show profile
    Profile {
        #[structopt()]
        /// Profile id to be shown, either yours or remote
        profile_id: Option<ProfileId>,

        #[structopt(long, default_value = "remote")]
        /// Source of the profile repository to be consulted for the lookup.
        /// Possible values are: local, base, remote
        source: ProfileRepositoryKind,
    },
}

#[async_trait(?Send)]
impl Command for ShowCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        match *self {
            ShowCommand::Profile { profile_id, source } => {
                let profile = api.get_profile_data(profile_id, source).await?;
                let public_profile = profile.public_data();
                let links = public_profile.links();
                let attributes = public_profile.attributes();

                info!("Details of profile id {}", public_profile.id());
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
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum CreateCommand {
    // TODO how to specify to keep current or new profile should be active/default
    #[structopt(name = "profile")]
    /// Create profile
    Profile {
        #[structopt()]
        /// Human-readable name of the new profile for easier identification
        label: Option<String>,
    },

    #[structopt(name = "link")]
    /// Create link, i.e. follow/subscribe to a remote profile
    Link {
        #[structopt(long)]
        /// Add link to this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt()]
        /// Create link to this remote profile
        peer_profile_id: ProfileId,
        // TODO is an optional "relation_type" needed here?
    },
}

#[async_trait(?Send)]
impl Command for CreateCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        use CreateCommand::*;
        match *self {
            Profile { label } => {
                //let profiles = api.list_vault_records()?;
                let profile = api.create_profile(label).await?;
                info!(
                    "Created and activated profile with label {}, id {}",
                    profile.label(),
                    profile.id()
                );
            }
            Link { my_profile_id, peer_profile_id } => {
                api.create_link(my_profile_id, &peer_profile_id).await?;
                info!("Created link to peer profile {}", peer_profile_id);
            }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum RemoveCommand {
    #[structopt(name = "link")]
    /// Remove link, i.e. unfollow/unsubscribe from another profile
    Link {
        #[structopt(long)]
        /// Remove link from this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt()]
        /// Remove link with this remote profile
        peer_profile_id: ProfileId,
    },
}

#[async_trait(?Send)]
impl Command for RemoveCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        match *self {
            RemoveCommand::Link { my_profile_id, peer_profile_id } => {
                api.remove_link(my_profile_id, &peer_profile_id).await?;
                info!("Removed link from profile {}", peer_profile_id);
            }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum SetCommand {
    #[structopt(name = "active_profile")]
    /// Show profile
    ActiveProfile {
        // TODO is activation by profile NUMBER needed or is this enough?
        #[structopt()]
        /// Profile id to be activated
        my_profile_id: ProfileId,
    },

    #[structopt(name = "attribute")]
    /// Set attribute with name to specified value
    Attribute {
        #[structopt(long)]
        /// Set attribute to this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt()]
        /// Attribute name
        key: AttributeId,

        #[structopt()]
        /// Attribute value
        value: AttributeValue,
    },
}

#[async_trait(?Send)]
impl Command for SetCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        use SetCommand::*;
        match *self {
            ActiveProfile { my_profile_id } => {
                api.set_active_profile(&my_profile_id).await?;
                info!("Active profile was set to {}", my_profile_id);
            }
            Attribute { my_profile_id, key, value } => {
                api.set_attribute(my_profile_id, &key, &value).await?;
                info!("Setting attribute {} to {}", key, value);
            }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum ClearCommand {
    #[structopt(name = "attribute")]
    /// Clear attribute
    Attribute {
        #[structopt(long)]
        /// Clear attribute from this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt()]
        /// Attribute name
        key: AttributeId,
    },
}

#[async_trait(?Send)]
impl Command for ClearCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        match *self {
            ClearCommand::Attribute { my_profile_id, key } => {
                api.clear_attribute(my_profile_id, &key).await?;
                info!("Cleared attribute: {}", key);
            }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum GenerateCommand {
    #[structopt(name = "vault")]
    /// Generate a phraselist needed to create a profile vault
    Vault,
}

#[async_trait(?Send)]
impl Command for GenerateCommand {
    async fn execute(self: Box<Self>, _api: &mut dyn VaultApi) -> CmdRes {
        match *self {
            GenerateCommand::Vault => {
                // TODO this should probably come from the daemon instead of generating it here
                show_generated_phrase();
                Ok(())
            }
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum RestoreCommand {
    #[structopt(name = "vault")]
    /// (Re)build a profile vault (needed for most commands) from a phraselist
    Vault {
        #[structopt(long)]
        demo: bool,
    },
    #[structopt(name = "profile")]
    /// Synchronize data of a profile from remote repository (possibly overwrite local data if exists)
    Profile {
        #[structopt()]
        /// Restore this specific profile from remote repository
        my_profile_id: Option<ProfileId>,

        #[structopt(long)]
        /// Enforce restoring remote profile version even if having conflicting local changes.
        force: bool,
    },
    #[structopt(name = "profiles")]
    /// Synchronize data of all profiles from remote repository (possibly overwrite local data if exists)
    Profiles,
}

#[async_trait(?Send)]
impl Command for RestoreCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        use RestoreCommand::*;
        match *self {
            Vault { demo } => {
                let phrase = if demo {
                    // TODO remove this hardcoded phrase, it should be either removed or be a constant near the API
                    "include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already".to_owned()
                } else {
                    read_phrase()?
                };
                api.restore_vault(phrase).await?;
                info!("Vault successfully initialized");
                let counts = api.restore_all_profiles().await?;
                info!(
                    "Tried {} profiles, successfully restored {}",
                    counts.try_count, counts.restore_count
                );
            }
            Profiles => {
                let counts = api.restore_all_profiles().await?;
                info!(
                    "Tried {} profiles, successfully restored {}",
                    counts.try_count, counts.restore_count
                );
            }
            Profile { my_profile_id, force } => {
                let profile = api.restore_profile(my_profile_id, force).await?;
                info!("Successfully restored profile {}", profile.id());
            }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum PublishCommand {
    #[structopt(name = "profile")]
    /// Publish local profile version to remote profile repository
    Profile {
        #[structopt()]
        /// Publish this specific local profile
        my_profile_id: Option<ProfileId>,

        #[structopt(long)]
        /// Enforce publishing local profile version even if having conflicting remote changes.
        force: bool,
    },
}

#[async_trait(?Send)]
impl Command for PublishCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        match *self {
            PublishCommand::Profile { my_profile_id, force } => {
                let profile_id = api.publish_profile(my_profile_id, force).await?;
                info!("Published profile {} to remote repository", profile_id);
            }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
pub enum RevertCommand {
    #[structopt(name = "profile")]
    /// Revert changes of modified but unpublished local profile version
    Profile {
        #[structopt()]
        /// Revert this specific local profile
        my_profile_id: Option<ProfileId>,
    },
}

#[async_trait(?Send)]
impl Command for RevertCommand {
    async fn execute(self: Box<Self>, api: &mut dyn VaultApi) -> CmdRes {
        match *self {
            RevertCommand::Profile { my_profile_id } => {
                let profile = api.revert_profile(my_profile_id).await?;
                info!(
                    "Reverted profile {} to last known remote version {}",
                    profile.id(),
                    profile.version()
                );
            }
        };
        Ok(())
    }
}
