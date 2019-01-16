use structopt::StructOpt;



#[derive(Debug, StructOpt)]
#[structopt(name = "prometheus", about = "Command line interface of Prometheus")]
pub enum Command
{
    #[structopt(name = "list")]
    /// List profiles or followers
    List(ListCommand),

    /// Show profile details
    Show(ShowCommand),

    #[structopt(name = "create")]
    /// Create profile or link
    Create(CreateCommand),

    #[structopt(name = "remove")]
    /// Remove link // TODO (or profile?)
    Remove(RemoveCommand),

    #[structopt(name = "set")]
    /// Set attribute
    Set(SetCommand),

    #[structopt(name = "Clear")]
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
    IncomingLinks,
}



#[derive(Debug, StructOpt)]
pub enum ShowCommand
{
    #[structopt(name = "profile")]
    /// Show profile
    Profile,
}



#[derive(Debug, StructOpt)]
pub enum CreateCommand
{
    #[structopt(name = "profile")]
    /// Create profile
    Profile,

    #[structopt(name = "link")]
    /// Create link
    Link,
}



#[derive(Debug, StructOpt)]
pub enum RemoveCommand
{
    #[structopt(name = "link")]
    /// Remove link
    Link,
}



#[derive(Debug, StructOpt)]
pub enum SetCommand
{
    #[structopt(name = "attribute")]
    /// Set attribute
    Attribute,
}



#[derive(Debug, StructOpt)]
pub enum ClearCommand
{
    #[structopt(name = "attribute")]
    /// Clear attribute
    Attribute,
}