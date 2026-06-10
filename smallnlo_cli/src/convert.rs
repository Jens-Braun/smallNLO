use color_eyre::Result;
use std::path::PathBuf;

use clap::Args;

use crate::util::{self, FileFormat};

#[derive(Args)]
pub(crate) struct ConvertArgs {
    /// input table to operate on
    file: PathBuf,
    #[arg(short, long, default_value = PathBuf::from("output").into_os_string())]
    /// name of created output file (extension is added depending on the output)
    outfile: PathBuf,
    /// strip scale dependence grids of the output file (only supported if exporting as SmallNLO table)
    #[arg(short, long, default_value_t = false)]
    strip: bool,
    #[arg(short, long, default_value_t = false)]
    /// compress the output file using zstd compression
    compress: bool,
    #[arg(short = 'l', long, default_value_t = 10)]
    /// zstd compression level to use
    compression_level: i32,
}

pub(crate) fn convert(args: &ConvertArgs) -> Result<()> {
    let format_in = FileFormat::probe_file(&args.file)?;
    let mut tab = crate::util::read_table(&args.file)?;
    if args.strip {
        tab.strip(None);
    }
    let format = if args.outfile.extension().is_none() {
        match format_in {
            FileFormat::SmallNLO => FileFormat::FastNLO,
            FileFormat::FastNLO => FileFormat::SmallNLO,
            FileFormat::CompressedSmallNLO => FileFormat::CompressedFastNLO,
            FileFormat::CompressedFastNLO => FileFormat::CompressedSmallNLO,
        }
    } else {
        FileFormat::probe_path(&args.outfile, args.compress)
    };
    util::write_table(&tab, &args.outfile, format, args.compression_level)?;
    return Ok(());
}
