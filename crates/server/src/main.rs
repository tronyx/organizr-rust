//! The `organizr` binary: start-up, scheduler, embedded web assets and subcommands.

mod check;
mod check_homepage;
mod cron;
mod healthcheck;
mod import;
mod restore;
mod scheduler;
mod secret_key;
mod serve;
mod web;

use clap::{Parser, Subcommand};

/// Command-line interface of the `organizr` binary.
#[derive(Debug, Parser)]
#[command(name = "organizr", version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the server.
    Serve(serve::Args),
    /// Check that a running server answers. Exits with an error when it does not.
    Healthcheck(healthcheck::Args),
    /// Move an install of PHP Organizr into this application. Run it once, before the
    /// first start of the server.
    Import(import::Args),
    /// Ask plex.tv, Ombi, Overseerr and Tautulli what a login asks them, with the
    /// stored settings. Prints for each one whether the answer was usable.
    Check(check::Args),
    /// Put the files and the database of a backup back. Stop the server first.
    Restore(restore::Args),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Serve(args) => serve::run(args).await,
        Command::Healthcheck(args) => healthcheck::run(&args).await,
        Command::Import(args) => import::run(args).await,
        Command::Check(args) => check::run(args).await,
        Command::Restore(args) => restore::run(&args),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
