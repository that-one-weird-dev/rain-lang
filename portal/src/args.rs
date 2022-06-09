use clap::{ArgEnum, Parser};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(arg_enum)]
    pub task: Task,

    #[clap(short, long)]
    pub release: bool,

    /// Module config file path
    #[clap(short, long, default_value="./portal.json")]
    pub module: String,
}

#[derive(ArgEnum, Clone, Debug)]
pub enum Task {
    Init,
    Build,
}