use color_eyre::{Result, eyre::Context};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;

use crate::util;

#[derive(Args)]
pub(crate) struct RecoArgs {
    /// smallNLO table to operate on
    file: PathBuf,
    #[arg(short, long, default_value = PathBuf::from("smallnlo_table.snlo").into_os_string())]
    /// name of created output file
    outfile: PathBuf,
    /// Parton distribution function (PDF) to use during reconstruction
    #[arg(short, long, default_value_t = String::from("PDF4LHC21_40"))]
    pdf: String,
    #[arg(short, long, default_value_t = false)]
    /// compress the output file using zstd compression
    compress: bool,
    #[arg(short = 'l', long, default_value_t = 10)]
    /// zstd compression level to use
    compression_level: i32,
}

pub(crate) fn reconstruct(args: &RecoArgs) -> Result<()> {
    let mut tab = util::read_table(&args.file)?;
    tab.reconstruct(&args.pdf, Box::new(Arc::new(|s1, _| s1)))
        .wrap_err("Error while reconstructing grids")?;
    crate::util::write_snlo(&tab, &args.outfile, args.compress, args.compression_level)
        .wrap_err("Error while writing output file")?;
    return Ok(());
}
