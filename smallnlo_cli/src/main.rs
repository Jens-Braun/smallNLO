use clap::{Parser, Subcommand, builder::styling};
use color_eyre::Result;
mod strip;

const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(version, about, styles = STYLES)]
#[command(propagate_version = true)]
struct CliConfig {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Strip scale-dependence grids from a fastNLO table
    Strip(strip::StripArgs),
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let conf = CliConfig::parse();
    return match &conf.command {
        Command::Strip(args) => strip::strip(args),
    };
}
