use clap::{Parser, Subcommand, builder::styling};
use color_eyre::{Result, eyre::eyre};
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::FmtSpan},
    prelude::*,
};
mod convert;
mod strip;
mod util;
mod xsec;

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
    /// Level at which to emit tracing information (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`)
    #[arg(long, default_value = "WARN", global = true)]
    loglevel: String,
}

#[derive(Subcommand)]
enum Command {
    /// Strip scale-dependence grids from a fastNLO table
    Strip(strip::StripArgs),
    /// Evaluate the cross section per bin
    #[command(name = "xsec")]
    XSec(xsec::XSecArgs),
    /// Convert FastNLO <-> SmallNLO tables
    Convert(convert::ConvertArgs),
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let conf = CliConfig::parse();
    let level_filter = match conf.loglevel.to_lowercase().as_str() {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "info" => LevelFilter::INFO,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        x => {
            return Err(eyre!(
                "Unknown loglevel `{x}`, expected `TRACE`, `DEBUG`, `INFO`, `WARN` or `ERROR`"
            ));
        }
    };
    tracing_subscriber::registry()
        .with(level_filter)
        .with(fmt::layer().pretty().with_span_events(FmtSpan::ACTIVE))
        .init();
    return match &conf.command {
        Command::Strip(args) => strip::strip(args),
        Command::XSec(args) => xsec::print_cross_section_table(args),
        Command::Convert(args) => convert::convert(args),
    };
}
