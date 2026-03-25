use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use rayon::prelude::*;
use std::path::PathBuf;

use clap::Args;

use crate::util::{read_table, write_snlo};

#[derive(Args)]
pub(crate) struct MergeArgs {
    /// input tables to operate on
    files: Vec<PathBuf>,
    #[arg(short, long, default_value = PathBuf::from("output").into_os_string())]
    /// name of created output file (extension is added depending on the output)
    outfile: PathBuf,
    /// Number of workers to use while merging
    #[arg(short, long, default_value = None)]
    jobs: Option<usize>,
    #[arg(short, long, default_value_t = false)]
    /// compress the output file using zstd compression
    compress: bool,
    #[arg(short = 'l', long, default_value_t = 10)]
    /// zstd compression level to use
    compression_level: i32,
}

pub(crate) fn merge(args: &MergeArgs) -> Result<()> {
    if let Some(j) = args.jobs {
        rayon::ThreadPoolBuilder::new().num_threads(j).build_global()?;
    }
    let res = args
        .files
        .par_iter()
        .map(|file| read_table(file).wrap_err("Unable to read table {file}"))
        .try_reduce_with(|mut t1, t2| {
            t1.merge(&t2).wrap_err("Incompatible tables while merging")?;
            Ok(t1)
        });
    if let Some(res) = res {
        write_snlo(&res?, &args.outfile, args.compress, args.compression_level)?;
    } else {
        return Err(eyre!("No input tables specified"));
    }
    return Ok(());
}
