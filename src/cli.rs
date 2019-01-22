use structopt::StructOpt;

use morpheus_storage::{AttributeId, AttributeValue};
use crate::types::{LinkId, ProfileId};



#[derive(Debug, StructOpt)]
#[structopt(name = "prometheus", about = "Command line interface of Prometheus")]
pub enum Command
{
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
}



#[derive(Debug, StructOpt)]
pub enum ListCommand
{
    #[structopt(name = "profiles")]
    /// List profiles
    Profiles,

    #[structopt(name = "followers")]
    /// List followers
    IncomingLinks
    {
        #[structopt(long = "my_profile_id")]
        /// List public followers of this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,
    },
}



#[derive(Debug, StructOpt)]
pub enum ShowCommand
{
    #[structopt(name = "profile")]
    /// Show profile
    Profile
    {
        #[structopt(long = "profile_id")]
        /// Profile id to be shown, either yours or remote
        profile_id: ProfileId,
    },
}



#[derive(Debug, StructOpt)]
pub enum CreateCommand
{
    #[structopt(name = "profile")]
    /// Create profile
    Profile, // TODO how to specify to keep current or new profile should be active/default

    #[structopt(name = "link")]
    /// Create link to a remote profile
    Link
    {
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
pub enum RemoveCommand
{
    #[structopt(name = "link")]
    /// Remove link
    Link
    {
        #[structopt(long = "my_profile_id")]
        /// Remove link from this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt(long = "link_id")]
        /// ID of link to be removed
        link_id: LinkId
    },
}



#[derive(Debug, StructOpt)]
pub enum SetCommand
{
    #[structopt(name = "active-profile")]
    /// Show profile
    ActiveProfile
    {
        // TODO is activation by profile NUMBER needed or is this enough?
        #[structopt(long = "my_profile_id")]
        /// Profile id to be activated
        my_profile_id: ProfileId,
    },

    #[structopt(name = "attribute")]
    /// Set attribute with name to specified value
    Attribute
    {
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
pub enum ClearCommand
{
    #[structopt(name = "attribute")]
    /// Clear attribute
    Attribute
    {
        #[structopt(long = "my_profile_id")]
        /// Clear attribute from this profile of yours if other than the active one
        my_profile_id: Option<ProfileId>,

        #[structopt(long = "key")]
        /// Attribute name
        key: AttributeId,
    }
}
