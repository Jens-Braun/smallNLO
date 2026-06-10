use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use clap::Args;

use crate::util::{FileFormat, read_table, write_table};

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
    /// path to `nnlojet-merge` weight file
    #[arg(short, long, default_value = None)]
    weights: Option<PathBuf>,
}

pub(crate) fn merge(args: &MergeArgs) -> Result<()> {
    if let Some(j) = args.jobs {
        rayon::ThreadPoolBuilder::new().num_threads(j).build_global()?;
    }
    let weight_map = if let Some(w) = &args.weights {
        read_nnlojet_weights(w)?
    } else {
        HashMap::new()
    };
    let res = args
        .files
        .par_iter()
        .map(|file| -> Result<_> {
            let mut tab = read_table(file).wrap_err("Unable to read table {file}")?;
            if args.weights.is_some() {
                let name = file.file_stem().unwrap().to_str().unwrap().to_owned();
                let name = name.trim_end_matches(".tab").to_owned();
                dbg!(&name);
                tab.set_merge_weights(weight_map.get(&name).unwrap());
            }
            Ok(tab)
        })
        .try_reduce_with(|mut t1, t2| {
            t1.merge(&t2, args.weights.is_some())
                .wrap_err("Incompatible tables while merging")?;
            Ok(t1)
        });
    if let Some(res) = res {
        write_table(
            &res?,
            &args.outfile,
            FileFormat::probe_path(&args.outfile, args.compress),
            args.compression_level,
        )?;
    } else {
        return Err(eyre!("No input tables specified"));
    }
    return Ok(());
}

fn read_nnlojet_weights(file: impl AsRef<Path>) -> Result<HashMap<String, Vec<f64>>, std::io::Error> {
    Ok(std::fs::read_to_string(file)?
        .lines()
        .map(|l| {
            if l.starts_with("#") {
                return None;
            }
            let mut elements = l.split_whitespace();
            let name = PathBuf::from(elements.next().unwrap())
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            dbg!(&name);
            let vals = elements.map(|e| e.parse::<f64>().unwrap()).collect();
            Some((name, vals))
        })
        .flatten()
        .collect())
}
