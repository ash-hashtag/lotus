use clap::{Args, CommandFactory, Parser, Subcommand};

// #[derive(Parser, Debug)]
// pub struct Args {
//     command: Commands,
// }

#[derive(Parser, Debug)]
pub struct Command {
    #[clap(subcommand)]
    commands: SubCommands,
}

#[derive(Subcommand, Debug)]
pub enum SubCommands {
    Pipeline { name: String },
    Noise(NoiseArgs),
    Set { name: String, value: String },
}

#[derive(Args, Debug)]
pub struct NoiseArgs {
    #[clap(short, long)]
    seed: f64,
    #[clap(short, long)]
    copy_to_texture: String,
}

impl Command {
    pub fn help_string() -> String {
        Self::command().render_long_help().to_string()
    }
}
