use crate::util::read_table;
use color_eyre::{Result, eyre::Context};
use std::path::PathBuf;

use clap::Args;

#[derive(Args)]
pub(crate) struct StripArgs {
    /// fastNLO table to operate on
    file: PathBuf,
    #[arg(short, long, default_value = PathBuf::from("smallnlo_table.snlo").into_os_string())]
    /// name of created output file
    outfile: PathBuf,
    #[arg(short, long, default_value_t = false)]
    /// compress the output file using zstd compression
    compress: bool,
    #[arg(short = 'l', long, default_value_t = 10)]
    /// zstd compression level to use
    compression_level: i32,
    #[arg(short, long)]
    /// Force the given scale format, completely purging all grids only present for `scale_format` > `force_scale_format`
    force_scale_format: Option<usize>,
}

pub(crate) fn strip(args: &StripArgs) -> Result<()> {
    let mut tab =
        read_table(&args.file).wrap_err_with(|| format!("Error while reading input table {:?}", args.file.clone()))?;
    tab.strip(args.force_scale_format);
    crate::util::write_snlo(&tab, &args.outfile, args.compress, args.compression_level)
        .wrap_err("Error while writing output file")?;
    return Ok(());
}
