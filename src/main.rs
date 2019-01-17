pub mod types;
pub mod vault;
pub mod cli;



fn main()
{
    use structopt::StructOpt;
    let config = cli::Command::from_args();
    println!("{:?}", config);
}