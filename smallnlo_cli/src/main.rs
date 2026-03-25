use clap::{CommandFactory, Parser, Subcommand, ValueEnum, builder::styling};
use color_eyre::Result;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::FmtSpan},
    prelude::*,
};
mod autocomplete;
mod convert;
mod merge;
mod reconstruct;
mod strip;
mod summary;
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
    /// Level at which to emit tracing information
    #[arg(long, value_enum, default_value_t = LogLevel::Warn, global = true)]
    loglevel: LogLevel,
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
    /// Merge FastNLO/SmallNLO tables into a single SmallNLO table by summing the grids
    Merge(merge::MergeArgs),
    /// Reconstruct previously stripped scale dependence grids
    Reconstruct(reconstruct::RecoArgs),
    /// Summarize the content of the input table
    Summary(summary::SummaryArgs),
    /// Generate shell autocompletion
    Autocompletion(autocomplete::AutocompleteArgs),
}

#[derive(ValueEnum, Clone)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let conf = CliConfig::parse();
    let level_filter = match conf.loglevel {
        LogLevel::Trace => LevelFilter::TRACE,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Error => LevelFilter::ERROR,
    };
    tracing_subscriber::registry()
        .with(level_filter)
        .with(fmt::layer().pretty().with_span_events(FmtSpan::ACTIVE))
        .init();
    return match &conf.command {
        Command::Strip(args) => strip::strip(args),
        Command::XSec(args) => xsec::print_cross_section_table(args),
        Command::Convert(args) => convert::convert(args),
        Command::Merge(args) => merge::merge(args),
        Command::Reconstruct(args) => reconstruct::reconstruct(args),
        Command::Summary(args) => summary::summary(args),
        Command::Autocompletion(args) => autocomplete::autocomplete(&mut CliConfig::command(), args),
    };
}
