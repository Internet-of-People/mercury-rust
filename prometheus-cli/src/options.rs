use std::net::SocketAddr;
use std::path::PathBuf;

use structopt::StructOpt;

use osg::model::*;

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
    pub command: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
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

impl Command {
    pub fn needs_vault(&self) -> bool {
        match self {
            Command::Generate(_) | Command::Restore(_) => false,
            Command::Show(ShowCommand::Profile { .. }) => false,
            _ => true,
        }
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

        #[structopt(long)]
        /// Profile id to be shown, either yours or remote
        local: bool,
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
    #[structopt(name = "profile")]
    /// Synchronize profile data from remote repository (possibly overwrite local data if exists)
    Profile {
        #[structopt(long = "my_profile_id")]
        /// Restore this specific profile from remote repository
        my_profile_id: Option<ProfileId>,
    },
}

#[derive(Debug, StructOpt)]
pub enum PublishCommand {
    #[structopt(name = "profile")]
    /// Publish local profile version to remote profile repository
    Profile {
        #[structopt(long = "my_profile_id")]
        /// Publish this specific local profile
        my_profile_id: Option<ProfileId>,
    },
}
