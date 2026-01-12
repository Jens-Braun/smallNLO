use color_eyre::{Result, eyre::Context};
use smallnlo::table::FastNLOFile;
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
}

pub(crate) fn strip(args: &StripArgs) -> Result<()> {
    let mut tab = FastNLOFile::read(args.file.clone())
        .wrap_err_with(|| format!("Error while reading fastNLO table {:?}", args.file.clone()))?;
    tab.strip();
    crate::util::write_snlo(&tab, &args.outfile, args.compress, args.compression_level)
        .wrap_err("Error while writing output file")?;
    return Ok(());
}
