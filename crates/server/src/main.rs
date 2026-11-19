//! The `organizr` binary: start-up, scheduler, embedded web assets and subcommands.

mod healthcheck;
mod import;
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Serve(args) => serve::run(args).await,
        Command::Healthcheck(args) => healthcheck::run(&args).await,
        Command::Import(args) => import::run(args).await,
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
