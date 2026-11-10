#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use ophi_core::{Error, MAX_RECORD_BYTES, NewCase, NewEntry, Repository, SqliteRepository};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "ophi",
    version,
    about = "Local-first, append-only exploration history"
)]
struct Cli {
    #[arg(long, global = true, default_value = "ophi.db")]
    db: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Case {
        #[command(subcommand)]
        command: CaseCommand,
    },
    Entry {
        #[command(subcommand)]
        command: EntryCommand,
    },
}

#[derive(Subcommand)]
enum CaseCommand {
    /// Create a Case; omit --title to read {"title":"..."} from stdin.
    Create {
        #[arg(long)]
        title: Option<String>,
    },
    Get {
        id: String,
    },
    History {
        case_id: String,
        #[arg(long)]
        after: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum EntryCommand {
    /// Read a NewEntry JSON object from stdin.
    Append,
    Get {
        id: String,
    },
}

fn input<T: DeserializeOwned>() -> Result<T, Error> {
    let mut bytes = Vec::new();
    io::stdin()
        .lock()
        .take((MAX_RECORD_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::Validation(format!("cannot read stdin: {e}")))?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(Error::Validation("input exceeds 1 MiB".into()));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn run(cli: Cli) -> Result<Value, Error> {
    // Parse write requests before opening or initializing the database.
    match cli.command {
        Command::Case {
            command: CaseCommand::Create { title },
        } => {
            let request = match title {
                Some(title) => NewCase { title },
                None => input()?,
            };
            Ok(serde_json::to_value(
                SqliteRepository::open(cli.db)?.create_case(request)?,
            )?)
        }
        Command::Entry {
            command: EntryCommand::Append,
        } => {
            let request: NewEntry = input()?;
            Ok(serde_json::to_value(
                SqliteRepository::open(cli.db)?.append(request)?,
            )?)
        }
        command => {
            let repo = SqliteRepository::open(cli.db)?;
            match command {
                Command::Case {
                    command: CaseCommand::Get { id },
                } => Ok(serde_json::to_value(
                    repo.get_case(&id)?
                        .ok_or(Error::NotFound { entity: "case", id })?,
                )?),
                Command::Case {
                    command:
                        CaseCommand::History {
                            case_id,
                            after,
                            limit,
                        },
                } => Ok(serde_json::to_value(repo.history(
                    &case_id,
                    after.as_deref(),
                    limit,
                )?)?),
                Command::Entry {
                    command: EntryCommand::Get { id },
                } => Ok(serde_json::to_value(repo.get(&id)?.ok_or(
                    Error::NotFound {
                        entity: "entry",
                        id,
                    },
                )?)?),
                _ => unreachable!(),
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            print!("{error}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!(
                "{}",
                json!({"protocol_version":1,"error":{"code":"invalid_arguments","message":error.to_string()}})
            );
            return ExitCode::from(2);
        }
    };
    match run(cli) {
        Ok(data) => {
            println!("{}", json!({"protocol_version":1,"data":data}));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                json!({"protocol_version":1,"error":{"code":error.code(),"message":error.to_string()}})
            );
            ExitCode::FAILURE
        }
    }
}
