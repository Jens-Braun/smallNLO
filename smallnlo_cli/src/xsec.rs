use color_eyre::{Result, eyre::Context};
use ndarray::prelude::*;
use smallnlo::{FastNLOEvalutator, Float};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tabled::{
    Table,
    settings::{Alignment, Remove, Span, Style, object::FirstRow, style::HorizontalLine, themes::BorderCorrection},
};

use clap::Args;

#[derive(Args, Debug)]
pub(crate) struct XSecArgs {
    /// fastNLO table to operate on
    file: PathBuf,
    /// Name of the parton distribution function (PDF) to use
    #[arg(short, long, default_value_t = String::from("PDF4LHC21_40"))]
    pdf: String,
    /// Second table to compare against the first
    #[arg(short, long, default_value = None)]
    compare: Option<PathBuf>,
    /// Order in perturbation theory up to which to calculate the cross section (i.e. 0 = LO, 1 = NLO, ...)
    #[arg(short, long, default_value = None)]
    order: Option<usize>,
    #[arg(short, long)]
    /// Fixed scale to evaluate the table at
    scale: Option<f64>,
}

pub fn print_cross_section_table(args: &XSecArgs) -> Result<()> {
    let (bins, xsec) = cross_section(&args.file, &args.pdf, args.order.clone(), args.scale.clone())?;
    let xsec_compare = if let Some(file2) = &args.compare {
        Some(
            cross_section(file2, &args.pdf, args.order.clone(), args.scale.clone())
                .wrap_err("Error while calculating cross sections of comparison file")?
                .1,
        )
    } else {
        None
    };
    let mut table = if let Some(ref cmp) = xsec_compare {
        Table::builder(bins.iter().zip(xsec.iter()).zip(cmp.iter()).enumerate().map(
            |(i, (((low, high), xs), xs_cmp))| {
                [
                    format!("{}", i + 1),
                    format!("{low:.2}"),
                    format!("{high:.2}"),
                    format!("{xs:.7E}"),
                    format!("{:.7E}", xs_cmp),
                    format!("{:.4}", 2. * (xs - xs_cmp).abs() / (xs + xs_cmp).abs()),
                ]
            },
        ))
    } else {
        Table::builder(bins.iter().zip(xsec.iter()).enumerate().map(|(i, ((low, high), xs))| {
            [
                format!("{}", i + 1),
                format!("{low:.2}"),
                format!("{high:.2}"),
                format!("{xs:.7E}"),
            ]
        }))
    };
    if let Some(cmp) = xsec_compare {
        table.insert_record(
            1,
            [
                "Bin Index",
                "Lower Edge",
                "Upper Edge",
                "dσ/dX",
                "dσᶜ/dX",
                "relative difference",
            ],
        );
        table.push_record([
            "Σ".to_owned(),
            "".into(),
            "".into(),
            format!("{:.7E}", xsec.sum()),
            format!("{:.7E}", cmp.sum()),
            format!(
                "{:.4}",
                2. * (xsec.sum() - cmp.sum()).abs() / (xsec.sum() + cmp.sum()).abs()
            ),
        ]);
    } else {
        table.insert_record(1, ["Bin Index", "Lower Edge", "Upper Edge", "dσ/dX"]);
        table.push_record(["Σ".to_owned(), "".into(), "".into(), format!("{:.7E}", xsec.sum())]);
    }

    println!(
        "{}",
        table
            .build()
            .modify((bins.len() + 1, 0), Span::column(3))
            .with(Style::rounded().horizontals([
                (1, HorizontalLine::inherit(Style::modern())),
                (bins.len() + 1, HorizontalLine::inherit(Style::modern()))
            ]))
            .with(Remove::row(FirstRow))
            .with(BorderCorrection::span())
            .with(Alignment::center())
    );
    return Ok(());
}

#[tracing::instrument(level = tracing::Level::DEBUG)]
fn cross_section(
    path: &Path,
    pdf: &str,
    order: Option<usize>,
    scale: Option<f64>,
) -> Result<(Vec<(f64, f64)>, Array1<Float>)> {
    let tab = crate::util::read_table(path).wrap_err("Error while reading input table")?;
    let mu = if let Some(s) = scale {
        Some(Box::new(
            Arc::new(move |s1, _| s * s1) as Arc<dyn Fn(f64, f64) -> f64 + Send + Sync>
        ))
    } else {
        None
    };
    let mut evaluator = FastNLOEvalutator::new(&tab, pdf, mu.clone(), mu);
    let xsec = evaluator
        .max_power(order)
        .cross_sections()
        .wrap_err("Error while calculating cross sections")?;
    let bins = tab
        .bin_info
        .bins
        .iter()
        .map(|b| match b.values[0] {
            smallnlo::table::BinPosition::Central(c) => (c - b.size / 2., c + b.size / 2.),
            smallnlo::table::BinPosition::Boundaries { low, high } => (low, high),
        })
        .collect();
    return Ok((bins, xsec));
}
