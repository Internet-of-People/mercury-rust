use std::net::SocketAddr;
use std::path::PathBuf;

use structopt::StructOpt;

use osg::api::{self, Api, ApiRes, ProfileRepositoryKind};
use osg::model::*;

pub trait Command {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes;
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
    /// Path of your local profile repository file. Default: OS-specific app_cfg_dir/prometheus/profiles.dat
    pub profile_repo_path: Option<PathBuf>,

    #[structopt(long = "bases", raw(value_name = r#""FILE""#), parse(from_os_str))]
    /// Path of the profile repository file caching the remote repository. Default: OS-specific app_cfg_dir/prometheus/bases.dat
    pub base_repo_path: Option<PathBuf>,

    #[structopt(
        long = "repository",
        default_value = "127.0.0.1:6161",
        raw(value_name = r#""IP:PORT""#)
    )]
    /// IPv4/6 address of the remote profile repository.
    pub remote_repo_address: SocketAddr,

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
    pub command: CommandVerb,
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
}

impl CommandVerb {
    pub fn needs_vault(&self) -> bool {
        use CommandVerb::*;
        match self {
            Generate(GenerateCommand::Vault { .. }) | Restore(RestoreCommand::Vault { .. }) => {
                false
            }
            Show(ShowCommand::Profile { .. }) => false,
            _ => true,
        }
    }
}

impl Command for CommandVerb {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use CommandVerb::*;
        let sub: Box<Command> = match *self {
            Generate(sub) => Box::new(sub),
            Restore(sub) => Box::new(sub),
            List(sub) => Box::new(sub),
            Show(sub) => Box::new(sub),
            Create(sub) => Box::new(sub),
            Remove(sub) => Box::new(sub),
            Set(sub) => Box::new(sub),
            Clear(sub) => Box::new(sub),
            Publish(sub) => Box::new(sub),
        };
        sub.execute(api)
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
        #[structopt()]
        /// List public followers of this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,
    },
}

impl Command for ListCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use ListCommand::*;
        match *self {
            Profiles => api.list_profiles(),
            IncomingLinks { my_profile_id } => api.list_incoming_links(my_profile_id),
        }
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

        #[structopt(default_value = "remote")]
        /// Source of the profile repository to be consulted for the lookup.
        /// Possible values are: local, base, remote
        source: ProfileRepositoryKind,
    },
}

impl Command for ShowCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use ShowCommand::*;
        match *self {
            Profile { profile_id, source } => api.show_profile(profile_id, source),
        }
    }
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

impl Command for CreateCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use CreateCommand::*;
        match *self {
            Profile => api.create_profile(),
            Link {
                my_profile_id,
                peer_profile_id,
            } => api.create_link(my_profile_id, peer_profile_id),
        }
    }
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

impl Command for RemoveCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use RemoveCommand::*;
        match *self {
            Link {
                my_profile_id,
                peer_profile_id,
            } => api.remove_link(my_profile_id, peer_profile_id),
        }
    }
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

impl Command for SetCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use SetCommand::*;
        match *self {
            ActiveProfile { my_profile_id } => api.set_active_profile(my_profile_id),
            Attribute {
                my_profile_id,
                key,
                value,
            } => api.set_attribute(my_profile_id, key, value),
        }
    }
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

impl Command for ClearCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use ClearCommand::*;
        match *self {
            Attribute { my_profile_id, key } => api.clear_attribute(my_profile_id, key),
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum GenerateCommand {
    #[structopt(name = "vault")]
    /// Generate a phraselist needed to create a profile vault
    Vault,
}

impl Command for GenerateCommand {
    fn execute(self: Box<Self>, _api: &mut Api) -> ApiRes {
        use GenerateCommand::*;
        match *self {
            Vault => {
                api::generate_vault();
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
        #[structopt(long = "demo")]
        demo: bool,
    },
    #[structopt(name = "profile")]
    /// Synchronize data of a profile from remote repository (possibly overwrite local data if exists)
    Profile {
        #[structopt()]
        /// Restore this specific profile from remote repository
        my_profile_id: Option<ProfileId>,
    },
    #[structopt(name = "profiles")]
    /// Synchronize data of all profiles from remote repository (possibly overwrite local data if exists)
    Profiles,
}

impl Command for RestoreCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use RestoreCommand::*;
        match *self {
            Vault { demo } => api.restore_vault(demo),
            Profiles => api.restore_all_profiles(),
            Profile { my_profile_id } => {
                api.pull_base_profile(my_profile_id.clone())?;
                // TODO detect and resolve conflicts here
                api.revert_local_profile_to_base(my_profile_id)
            }
        }
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
    },
}

impl Command for PublishCommand {
    fn execute(self: Box<Self>, api: &mut Api) -> ApiRes {
        use PublishCommand::*;
        match *self {
            Profile { my_profile_id } => {
                // TODO detect and resolve conflicts here
                api.push_local_profile(my_profile_id.clone())?;
                api.pull_base_profile(my_profile_id)
            }
        }
    }
}
