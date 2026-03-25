use crate::util::read_table;
use clap::Args;
use color_eyre::Result;
use owo_colors::{OwoColorize, colors::CustomColor};
use smallnlo::table::BinPosition;
use std::path::PathBuf;
use tabled::{
    builder::Builder,
    settings::{Alignment, Span, Style, themes::BorderCorrection},
};

#[derive(Args)]
pub(crate) struct SummaryArgs {
    /// Input table to operate on
    file: PathBuf,
}

pub fn summary(args: &SummaryArgs) -> Result<()> {
    let table = read_table(&args.file)?;
    println!(
        "{:-^113}\n",
        format!(
            " Summary of scenario {} {} ",
            table.metadata.scenario_name.to_string().red(),
            format!("[v{}]", table.metadata.table_version).fg::<CustomColor<128, 128, 128>>()
        )
    );
    println!(
        "\n--------------------------------- {} ---------------------------------\n",
        "Description".bold(),
    );
    println!("{}", table.metadata.description.join("\n").italic());
    println!(
        "\n----------------------------------- {} ----------------------------------\n",
        "Metadata".bold(),
    );
    println!(
        "Number of contributions: {} ({} multiplicative, {} data, {} user)",
        table.metadata.n_contrib.red(),
        table.metadata.n_mult.cyan(),
        table.metadata.n_mult.cyan(),
        table.metadata.n_user.cyan()
    );
    println!("Fundamental power in αₛ: {}", table.metadata.alphas_ord.red());
    println!("Center-of-mass energy:   {} GeV", table.metadata.cms_energy.red());
    println!(
        "\n----------------------------------- {} ----------------------------------\n",
        "Bin Info".bold(),
    );
    println!("Number of bins:       {}", table.bin_info.bins.len().red());
    println!(
        "Dimension of binning: {} {}",
        table.bin_info.dim_labels.len().red(),
        format!("[{}]", table.bin_info.dim_labels.join(", ")).fg::<CustomColor<128, 128, 128>>()
    );
    let mut table_builder = Builder::default();
    table_builder.push_record(
        vec!["Bin Index".to_owned()]
            .into_iter()
            .chain(table.bin_info.dim_labels.iter().cloned()),
    );
    for (i, b) in table.bin_info.bins.iter().enumerate() {
        table_builder.push_record(
            [(i + 1).to_string()].into_iter().chain(
                b.values
                    .iter()
                    .flat_map(|v| match v {
                        BinPosition::Boundaries { low, high } => [(*low).to_string(), (*high).to_string()],
                        BinPosition::Central(c) => [(*c - b.size / 2.).to_string(), (*c + b.size / 2.).to_string()],
                    })
                    .collect::<Vec<_>>(),
            ),
        );
    }
    let mut styled_table = table_builder.build();
    for i in 0..table.bin_info.dim_labels.len() {
        styled_table.modify((0, 1 + i), Span::column(2));
    }
    println!(
        "{}",
        styled_table
            .with(Style::rounded())
            .with(BorderCorrection::span())
            .with(Alignment::center())
    );

    for (i, b) in table.blocks.iter().enumerate() {
        match &b.data {
            smallnlo::table::BlockData::TheoryBlock {
                alphas_power,
                weight_info,
                n_events,
                pdf_info,
                x1_nodes,
                x2_nodes,
                grid,
                ..
            } => {
                let order = match alphas_power - table.metadata.alphas_ord {
                    0 => "LO",
                    1 => "NLO",
                    2 => "NNLO",
                    3 => "N³LO",
                    _ => todo!(),
                };
                println!("\n{:-^99}\n", format!(" Block {} [{}] ", i.cyan(), order.red()).bold());
                println!("Order in αₛ:                {}", alphas_power.red());
                println!(
                    "Number of entries in table: {}",
                    (if let Some(w) = weight_info {
                        w.n_entries
                    } else {
                        (*n_events) as usize
                    })
                    .red()
                );
                if let Some(w) = weight_info {
                    println!("Number of events in table:  {}", format!("{:E}", w.n_events).red());
                    println!("Normalization:              {}", format!("{:E}", w.norm).red());
                }
                println!("Number of subprocesses:     {}", pdf_info.n_subproc.red());
                println!(
                    "Number of nodes in x₁:      {} * {}",
                    x1_nodes.len().red(),
                    if !x1_nodes.is_empty() { x1_nodes[0].len() } else { 0 }.red()
                );
                println!(
                    "Number of nodes in x₂:      {} * {}",
                    x2_nodes.len().red(),
                    if !x2_nodes.is_empty() { x2_nodes[0].len() } else { 0 }.red()
                );
                match grid {
                    smallnlo::table::Grid::Flex {
                        scale_node_1,
                        scale_node_2,
                        grid,
                        grid_f,
                        grid_r,
                        grid_rr,
                        grid_ff,
                        grid_rf,
                        ..
                    } => {
                        println!(
                            "Number of nodes in scale 1: {} * {}",
                            scale_node_1.dim().0.red(),
                            scale_node_1.dim().1.red()
                        );
                        println!(
                            "Number of nodes in scale 2: {} * {}",
                            scale_node_2.dim().0.red(),
                            scale_node_2.dim().1.red()
                        );
                        println!("Number of entries per grid: {}", grid.len().red());
                        println!("Scale format:               {}", b.scale_format.red());
                        println!(
                            "First central grid entry:   {:E}",
                            grid.iter().find(|x| **x != 0.).unwrap_or(&0.).red()
                        );
                        let mut grids = vec!["Central".red().to_string()];
                        if grid_r.is_some() {
                            println!(
                                "First R grid entry:         {:E}",
                                grid_r.as_ref().unwrap().iter().find(|x| **x != 0.).unwrap_or(&0.).red()
                            );
                            grids.push("R".red().to_string());
                        }
                        if grid_f.is_some() {
                            println!(
                                "First F grid entry:         {:E}",
                                grid_f.as_ref().unwrap().iter().find(|x| **x != 0.).unwrap_or(&0.).red()
                            );
                            grids.push("F".red().to_string());
                        }
                        if grid_rr.is_some() {
                            println!(
                                "First RR grid entry:        {:E}",
                                grid_rr
                                    .as_ref()
                                    .unwrap()
                                    .iter()
                                    .find(|x| **x != 0.)
                                    .unwrap_or(&0.)
                                    .red()
                            );
                            grids.push("RR".red().to_string());
                        }
                        if grid_ff.is_some() {
                            println!(
                                "First FF grid entry:        {:E}",
                                grid_ff
                                    .as_ref()
                                    .unwrap()
                                    .iter()
                                    .find(|x| **x != 0.)
                                    .unwrap_or(&0.)
                                    .red()
                            );
                            grids.push("FF".red().to_string());
                        }
                        if grid_rf.is_some() {
                            println!(
                                "First RF grid entry:        {:E}",
                                grid_rf
                                    .as_ref()
                                    .unwrap()
                                    .iter()
                                    .find(|x| **x != 0.)
                                    .unwrap_or(&0.)
                                    .red()
                            );
                            grids.push("RF".red().to_string());
                        }
                        println!("Present grids:              [{}]", grids.join(", "));
                    }
                    _ => todo!(),
                }
            }
            _ => todo!(),
        }
    }

    Ok(())
}
