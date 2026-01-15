use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use std::io::{Read, Write};
use std::path::Path;

use bitcode;
use flate2::bufread::GzDecoder;
use smallnlo::table::FastNLOFile;

pub(crate) enum FileFormat {
    FastNLO,
    CompressedFastNLO,
    SmallNLO,
    CompressedSmallNLO,
}

impl FileFormat {
    pub(crate) fn probe_file(path: &Path) -> Result<Self> {
        let mut file = std::fs::File::open(path).wrap_err("Unable to open input table")?;
        let mut buf = [0u8; 4];
        file.read_exact(&mut buf)
            .wrap_err("Unable to read data from input table")?;
        drop(file);
        tracing::debug!(?buf);
        return Ok(match buf {
            // GZip magic number
            [0x1f, 0x8b, _, _] => Self::CompressedFastNLO,
            // FastNLO magic number
            [0x31, 0x32, 0x33, 0x34] => Self::FastNLO,
            // ZStd magic number
            [0x28, 0xb5, 0x2f, 0xfd] => Self::CompressedSmallNLO,
            // SmallNLO magic number
            [0x7c, 0x6e, 0x6c, 0x6f] => Self::SmallNLO,
            _ => {
                return Err(eyre!("Unable to identify input table as one of the supported formats"));
            }
        });
    }
}

pub(crate) fn write_snlo(table: &FastNLOFile, path: &Path, compress: bool, level: i32) -> Result<()> {
    let mut tab_ser = bitcode::serialize(table).wrap_err("Error while serializing FastNLO table")?;
    let mut outfile = std::fs::File::create(path).wrap_err("Error while opening the output file")?;
    if compress {
        tab_ser =
            zstd::encode_all(tab_ser.as_slice(), level).wrap_err("Error while compressing serialized FastNLO table")?;
        outfile
            .write_all(&tab_ser)
            .wrap_err("Error while writing to the output file")?;
    } else {
        // Add `snlo` magic number
        outfile
            .write_all(&[0x7c, 0x6e, 0x6c, 0x6f])
            .wrap_err("Error while writing to the output file")?;
        outfile
            .write_all(&tab_ser)
            .wrap_err("Error while writing to the output file")?;
    }
    return Ok(());
}

pub(crate) fn read_table(path: &Path) -> Result<FastNLOFile> {
    let mut file =
        std::fs::File::open(path).wrap_err_with(|| format!("Unable to open input table `{}`", path.display()))?;
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)
        .wrap_err_with(|| format!("Unable to read data from input table `{}`", path.display()))?;
    drop(file);
    let tab = match buf {
        // GZip magic number
        [0x1f, 0x8b, _, _] => {
            tracing::info!("Reading compressed FastNLO table `{path:?}`");
            let mut buf = String::new();
            GzDecoder::new(std::fs::read(path).unwrap().as_slice())
                .read_to_string(&mut buf)
                .wrap_err("Unable to read decompress input table")?;
            FastNLOFile::read_str(&buf)?
        }
        // FastNLO magic number
        [0x31, 0x32, 0x33, 0x34] => {
            tracing::info!("Reading FastNLO table `{path:?}`");
            FastNLOFile::read(path.into())?
        }
        // ZStd magic number
        [0x28, 0xb5, 0x2f, 0xfd] => {
            tracing::info!("Reading compressed SmallNLO table `{path:?}`");
            bitcode::deserialize(
                &zstd::decode_all(std::fs::read(path).wrap_err("Unable to open input table")?.as_slice())
                    .wrap_err("Unable to decompress input table")?,
            )
            .wrap_err("Unable to deserialize decompressed input table")?
        }
        // SmallNLO magic number
        [0x7c, 0x6e, 0x6c, 0x6f] => {
            tracing::info!("Reading SmallNLO table `{path:?}`");
            bitcode::deserialize(&std::fs::read(path).wrap_err("Unable to open input table")?[4..])
                .wrap_err("Unable to deserialize input table")?
        }
        _ => {
            return Err(eyre!("Unable to identify input table as one of the supported formats"));
        }
    };
    return Ok(tab);
}
