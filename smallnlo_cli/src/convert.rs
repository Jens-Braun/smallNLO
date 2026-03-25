use color_eyre::{Result, eyre::Context};
use flate2::bufread::GzEncoder;
use std::io::Read;
use std::path::PathBuf;

use clap::Args;

use crate::util::FileFormat;

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
    let format = FileFormat::probe_file(&args.file)?;
    let mut tab = crate::util::read_table(&args.file)?;
    match format {
        FileFormat::SmallNLO | FileFormat::CompressedSmallNLO => {
            if args.compress {
                let mut buf = String::new();
                tab.write(&mut buf)?;
                let mut gz = GzEncoder::new(buf.as_bytes(), flate2::Compression::best());
                let mut out = args.outfile.clone();
                out.add_extension("gz");
                let mut byte_buf = Vec::new();
                gz.read_to_end(&mut byte_buf)?;
                std::fs::write(out, &byte_buf)?;
            } else {
                tab.write_file(&args.outfile)?;
            }
        }
        FileFormat::CompressedFastNLO | FileFormat::FastNLO => {
            tracing::debug!("Input table is FastNLO, exporting SmallNLO");
            if args.strip {
                tracing::debug!("Stripping scale dependence grids");
                tab.strip(None);
            }
            let mut out_path = args.outfile.clone();
            out_path.set_extension("snlo");
            crate::util::write_snlo(&tab, &out_path, args.compress, args.compression_level)
                .wrap_err("Error while writing output file")?;
        }
    }
    return Ok(());
}
