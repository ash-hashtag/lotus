use clap::{Args, CommandFactory, Parser, Subcommand};
use log::{error, warn};

use crate::state::State;

#[derive(Parser, Debug)]
pub struct Command {
    #[clap(subcommand)]
    commands: SubCommands,
}

#[derive(Subcommand, Debug)]
pub enum SubCommands {
    Pipeline {
        name: String,
    },
    Noise(NoiseArgs),
    Set {
        name: String,
        value: String,
    },
    Spawn {
        name: String,
        mesh: String,
    },
    Scale {
        name: String,
        x: f32,
        y: f32,
        z: f32,
    },
    Position {
        name: String,
        x: f32,
        y: f32,
        z: f32,
    },
    Rotation {
        name: String,
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    },
    HelpMe,
}

#[derive(Args, Debug)]
pub struct NoiseArgs {
    #[clap(short, long)]
    seed: f64,
    #[clap(short, long)]
    copy_to_texture: String,
}

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("Is Not A Command, maybe message?")]
    NotACommand,
    #[error("MisMatchedQuotes {0:?}")]
    ShellWordSplitError(#[from] shellwords::MismatchedQuotes),
    #[error("Command Line Parse Error {0:?}")]
    ClapParseError(#[from] clap::Error),
}

impl Command {
    fn help_string() -> String {
        Self::command().render_long_help().to_string()
    }

    pub fn parse(cmd: &str) -> Result<Self, CommandError> {
        if !cmd.starts_with("lotus") {
            return Err(CommandError::NotACommand);
        }
        let words = shellwords::split(cmd)?;
        let result = Self::try_parse_from(words)?;
        Ok(result)
    }

    pub fn command_to_state(command: &str, state: &mut State) {
        match Self::parse(command) {
            Ok(cmd) => match cmd.commands {
                SubCommands::Pipeline { name } => todo!(),
                SubCommands::Noise(noise_args) => todo!(),
                SubCommands::Set { name, value } => todo!(),
                SubCommands::Spawn { name, mesh } => {}
                SubCommands::Position { name, x, y, z } => todo!(),
                SubCommands::Rotation { name, x, y, z, w } => todo!(),
                SubCommands::HelpMe => {
                    state.console_node.add_to_history(&Self::help_string());
                }
                _ => todo!(),
            },
            Err(err) => {
                match err {
                    CommandError::NotACommand => {}
                    _ => warn!("{:?}", err),
                };
            }
        };
    }
}
