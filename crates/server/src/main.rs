//! The `organizr` binary: start-up, scheduler, embedded web assets and subcommands.

use clap::Parser;

/// Command-line interface of the `organizr` binary.
#[derive(Debug, Parser)]
#[command(name = "organizr", version, about)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
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
