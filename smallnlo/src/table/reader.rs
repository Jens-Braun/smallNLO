use ndarray::prelude::*;

use crate::error::ReadError;
use crate::table::{
    Bin, BinInfo, Block, BlockData, FastNLOFile, Float, Grid, Metadata, PDFInfo, UserBlock,
    WeightInfo,
};
use std::{any::type_name, path::PathBuf, str::FromStr};

const SEP: &str = "1234567890";

pub(crate) fn read_fastnlo(file: PathBuf) -> Result<FastNLOFile, ReadError> {
    let content = std::fs::read_to_string(file)?;
    return read_fastnlo_str(&content);
}

#[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
pub(crate) fn read_fastnlo_str(content: &str) -> Result<FastNLOFile, ReadError> {
    let mut line_iter = content.lines();
    let lines = &mut line_iter;

    // -------------------- METADATA --------------------
    // ---- Block A1 ----
    assert_eq!(*lines.next().unwrap(), *SEP);
    let table_version = take::<usize>(lines)?;
    let scenario_name = take::<String>(lines)?;
    let n_contrib = take::<usize>(lines)?;
    let n_mult = take::<usize>(lines)?;
    let n_data = take::<usize>(lines)?;
    let n_user = take::<usize>(lines)?;
    let mut user_blocks = Vec::with_capacity(n_user);
    for _ in 0..n_user {
        let user_flag = take::<usize>(lines)?;
        let description = take_multiple::<String>(lines, None)?;
        let lines = take_multiple::<String>(lines, None)?;
        user_blocks.push(UserBlock {
            user_flag,
            description,
            lines,
        });
    }
    // Table contains some unknown numbers here, so we skip to the next SEP
    let mut line_iter = line_iter.skip_while(|line| **line != *SEP);
    let lines = &mut line_iter;
    assert_eq!(*lines.next().unwrap(), *SEP);

    // ---- Block A2 ----
    let unit = take::<usize>(lines)?;
    let description = take_multiple::<String>(lines, None)?;
    let cms_energy = take::<Float>(lines)?;
    let alphas_ord = take::<usize>(lines)?;
    tracing::info!(
        "Successfully read metadata of FastNLO table (v{table_version}) for scenario `{scenario_name}` containing `{}` blocks",
        n_data + n_contrib
    );
    let metadata = Metadata {
        table_version,
        scenario_name,
        n_contrib,
        n_mult,
        n_data,
        n_user,
        unit,
        description,
        cms_energy,
        alphas_ord,
    };
    tracing::debug!(?metadata, "Successfully read table metadata");

    let n_bins = take::<usize>(lines)?;
    let n_dim = take::<usize>(lines)?;
    let _dim_labels = take_multiple::<String>(lines, Some(n_dim))?;
    let diff_bin = take_multiple::<usize>(lines, Some(n_dim))?;
    let mut bin_infos = Vec::with_capacity(n_bins);
    for _ in 0..n_bins {
        let mut values = Vec::with_capacity(n_dim);
        for j in 0..n_dim {
            if diff_bin[j] == 1 {
                values.push(BinInfo::Central(take::<Float>(lines)?));
            } else if diff_bin[j] == 2 {
                values.push(BinInfo::Boundaries {
                    low: take::<Float>(lines)?,
                    high: take::<Float>(lines)?,
                });
            } else {
                unreachable!();
            }
        }
        bin_infos.push(values);
    }
    let bin_sizes = take_multiple::<Float>(lines, Some(n_bins))?;
    let bins = bin_infos
        .into_iter()
        .zip(bin_sizes.into_iter())
        .map(|(values, size)| Bin { values, size })
        .collect::<Vec<_>>();
    tracing::info!(
        "Successfully read {n_dim}-dimensional binning with {n_bins} bins per dimension"
    );
    tracing::debug!(?bins, "Successfully read binning");

    let norm_flag = take::<isize>(lines)?;
    let mut _denom_table = None;
    let mut denom_bins = Vec::with_capacity(n_dim);
    if norm_flag < 0 {
        _denom_table = Some(take::<String>(lines)?);
    }
    if norm_flag != 0 {
        for _ in 0..n_dim {
            denom_bins.push((take::<isize>(lines)?, take::<isize>(lines)?))
        }
    }

    // -------------------- DATA BLOCKS --------------------
    let mut blocks = Vec::with_capacity(n_contrib + n_data);
    for b in 0..(n_contrib + n_data) {
        assert_eq!(*lines.next().unwrap(), *SEP);
        let unit = take::<usize>(lines)?;
        let data_block = take::<usize>(lines)? == 1;
        let mult_block = take::<usize>(lines)? == 1;
        let contribution_type = take::<usize>(lines)?;
        let contribution_order = take::<usize>(lines)?;
        let scale_format = take::<usize>(lines)?;
        let description = take_multiple::<String>(lines, None)?;
        let code_description = take_multiple::<String>(lines, None)?;

        let data = if data_block {
            tracing::info!("Reading data of block {b} (data block): \n        {description:?}");
            read_data_block(lines.by_ref(), n_bins)?
        } else if mult_block {
            tracing::info!(
                "Reading data of block {b} (multiplicative contribution block): \n        {description:?}"
            );
            read_mult_block(lines.by_ref(), n_bins)?
        } else {
            tracing::info!("Reading data of block {b} (theory block): \n        {description:?}");
            read_theory_block(lines.by_ref(), n_bins, scale_format)?
        };
        let mut coeff_info_flags_1;
        let mut coeff_info_flags_2;
        let mut coeff_block_description;
        let mut coeff_block_content;
        if table_version >= 25000 {
            let n_info_blocks = take::<usize>(lines)?;
            coeff_info_flags_1 = Vec::with_capacity(n_info_blocks);
            coeff_info_flags_2 = Vec::with_capacity(n_info_blocks);
            coeff_block_description = Vec::with_capacity(n_info_blocks);
            coeff_block_content = Vec::with_capacity(n_info_blocks);
            for _ in 0..n_info_blocks {
                coeff_info_flags_1.push(take::<usize>(lines)?);
                coeff_info_flags_2.push(take::<usize>(lines)?);
                coeff_block_description.push(take_multiple::<String>(lines, None)?);
                coeff_block_content.push(take_multiple::<Float>(lines, None)?);
            }
        } else {
            coeff_info_flags_1 = Vec::new();
            coeff_info_flags_2 = Vec::new();
            coeff_block_description = Vec::new();
            coeff_block_content = Vec::new();
        }
        blocks.push(Block {
            unit,
            data_block,
            mult_block,
            contribution_type,
            contribution_order,
            scale_format,
            description,
            code_description,
            data,
            coeff_info_flags_1,
            coeff_info_flags_2,
            coeff_block_description,
            coeff_block_content,
        });
    }

    assert_eq!(*lines.next().unwrap(), *SEP);
    assert_eq!(*lines.next().unwrap(), *SEP);
    tracing::info!("Successfully read FastNLO table");
    return Ok(FastNLOFile {
        metadata,
        bins,
        blocks,
    });
}

#[inline]
#[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
fn read_data_block<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    n_bins: usize,
) -> Result<BlockData, ReadError> {
    let n_uncorr = take::<usize>(lines)?;
    let uncorr_sources = take_multiple::<String>(lines, Some(n_uncorr))?;
    let n_corr = take::<usize>(lines)?;
    let corr_sources = take_multiple::<String>(lines, Some(n_corr))?;
    let mut centers = Array1::zeros(n_bins);
    let mut values = Array1::zeros(n_bins);
    let mut uncorr_low = Array2::zeros((n_bins, n_uncorr));
    let mut uncorr_high = Array2::zeros((n_bins, n_uncorr));
    let mut corr_low = Array2::zeros((n_bins, n_corr));
    let mut corr_high = Array2::zeros((n_bins, n_corr));
    for i in 0..n_bins {
        centers[i] = take::<Float>(lines)?;
        values[i] = take::<Float>(lines)?;
        for j in 0..n_uncorr {
            uncorr_low[[i, j]] = take::<Float>(lines)?;
            uncorr_high[[i, j]] = take::<Float>(lines)?;
        }
        for j in 0..n_corr {
            corr_low[[i, j]] = take::<Float>(lines)?;
            corr_high[[i, j]] = take::<Float>(lines)?;
        }
    }
    let n_mat = take::<usize>(lines)?;
    let corr_matrix =
        Array2::from_shape_simple_fn((n_mat, n_bins * n_bins), || take::<Float>(lines).unwrap());

    return Ok(BlockData::DataBlock {
        uncorr_sources,
        corr_sources,
        centers,
        values,
        uncorr_low,
        uncorr_high,
        corr_low,
        corr_high,
        corr_matrix,
    });
}

#[inline]
#[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
fn read_mult_block<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    n_bins: usize,
) -> Result<BlockData, ReadError> {
    let n_uncorr = take::<usize>(lines)?;
    let uncorr_sources = take_multiple::<String>(lines, Some(n_uncorr))?;
    let n_corr = take::<usize>(lines)?;
    let corr_sources = take_multiple::<String>(lines, Some(n_corr))?;
    let mut centers = Array1::zeros(n_bins);
    let mut values = Array1::zeros(n_bins);
    let mut uncorr_low = Array2::zeros((n_bins, n_uncorr));
    let mut uncorr_high = Array2::zeros((n_bins, n_uncorr));
    let mut corr_low = Array2::zeros((n_bins, n_corr));
    let mut corr_high = Array2::zeros((n_bins, n_corr));
    for i in 0..n_bins {
        centers[i] = take::<Float>(lines)?;
        values[i] = take::<Float>(lines)?;
        for j in 0..n_uncorr {
            uncorr_low[[i, j]] = take::<Float>(lines)?;
            uncorr_high[[i, j]] = take::<Float>(lines)?;
        }
        for j in 0..n_corr {
            corr_low[[i, j]] = take::<Float>(lines)?;
            corr_high[[i, j]] = take::<Float>(lines)?;
        }
    }

    return Ok(BlockData::MultBlock {
        uncorr_sources,
        corr_sources,
        centers,
        values,
        uncorr_low,
        uncorr_high,
        corr_low,
        corr_high,
    });
}

#[inline]
#[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
fn read_theory_block<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    n_bins: usize,
    scale_dependence: usize,
) -> Result<BlockData, ReadError> {
    let reference_table = take::<usize>(lines)? == 1;
    let _i_scale_dependence = take::<usize>(lines)?;
    let n_events = take::<isize>(lines)?;
    let weight_info;
    if n_events < 0 {
        let _n_events_2 = take::<Float>(lines)?;
        let norm = take::<Float>(lines)?;
        let n_tables = if n_events <= -2 {
            take::<usize>(lines)?
        } else {
            1
        };
        let n_entries = take::<usize>(lines)?;
        let sum_weights_sq = take::<Float>(lines)?;
        let sum_sig_sq: Float = take::<Float>(lines)?;
        let sum_sig: Float = take::<Float>(lines)?;
        let weight_sq_obs = take_nested_vec::<Float>(lines, None)?;
        let sig_sq_obs = take_nested_vec::<Float>(lines, None)?;
        let sig_obs = take_nested_vec::<Float>(lines, None)?;
        let n_events_obs = take_nested_vec::<usize>(lines, None)?;
        weight_info = Some(WeightInfo {
            norm,
            n_tables,
            n_entries,
            sum_weights_sq,
            sum_sig_sq,
            sum_sig,
            weight_sq_obs,
            sig_sq_obs,
            sig_obs,
            n_events_obs,
        });
        tracing::info!(
            "Found full weight info with normalization `{norm}` and {n_entries} entries"
        );
    } else {
        weight_info = None;
    }
    let alphas_power = take::<usize>(lines)?;
    let pdf_info = read_pdf_info(lines)?;

    // The following is mentioned in the table format specification, but not in the fastnlotktoolkit
    // let mut n_events_bins = Vec::with_capacity(n_bins);
    // for _ in 0..n_bins {
    // n_events_bins.push(take_multiple::<usize>(lines, Some(pdf_info.n_subproc))?);
    // }
    if pdf_info.pdf_flag_1 == 0 {
        todo!();
    }
    let x1_nodes = take_nested_vec::<Float>(lines, Some(n_bins))?;
    tracing::info!(
        "Found x₁ grid with {} nodes for the first bin",
        x1_nodes[0].len()
    );
    let x2_nodes = if pdf_info.n_pdf_dim == 2 {
        take_nested_vec::<Float>(lines, Some(n_bins))?
    } else {
        Vec::new()
    };
    if !x2_nodes.is_empty() {
        tracing::info!(
            "Found x₂ grid with {} nodes for the second bin",
            x2_nodes[0].len()
        );
    }
    let z_nodes = if pdf_info.n_ff_dim > 0 {
        take_nested_vec::<Float>(lines, Some(n_bins))?
    } else {
        Vec::new()
    };
    let n_scales = take::<usize>(lines)?;
    let n_scale_dim = take::<usize>(lines)?;
    let scale_dimension = take_multiple::<usize>(lines, Some(n_scales))?;
    let scale_description = take_nested_vec::<String>(lines, Some(n_scale_dim))?;
    let grid = read_grid(
        lines,
        if let Some(ref w) = weight_info {
            w.norm
        } else {
            n_events as Float
        },
        scale_dependence,
        n_bins,
        n_scale_dim,
        pdf_info.n_subproc,
        pdf_info.n_pdf_dim,
        &x1_nodes,
        &x2_nodes,
    )?;
    return Ok(BlockData::TheoryBlock {
        reference_table,
        scale_dependence,
        n_events,
        weight_info,
        alphas_power,
        pdf_info,
        //n_events_bins,
        x1_nodes,
        x2_nodes,
        z_nodes,
        scale_dimension,
        scale_description,
        grid,
    });
}

#[inline]
#[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
fn read_pdf_info<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Result<PDFInfo, ReadError> {
    let pdfs = take_multiple::<usize>(lines, None)?;
    let n_pdf_dim = take::<usize>(lines)?;
    let fragmentation_functions = take_multiple::<usize>(lines, None)?;
    let n_ff_dim = take::<usize>(lines)?;
    let n_subproc = take::<usize>(lines)?;
    let pdf_flag_1 = take::<usize>(lines)?;
    let pdf_flag_2 = take::<usize>(lines)?;
    let pdf_flag_3 = take::<usize>(lines)?;
    let mut parton_flavors;
    if pdf_flag_2 == 0 {
        let coeff_format = take::<usize>(lines)?;
        if coeff_format == 0 {
            parton_flavors = Vec::with_capacity(n_subproc);
            for _ in 0..n_subproc {
                let n_pairs = take::<usize>(lines)?;
                let mut pairs = Vec::with_capacity(n_pairs);
                for _ in 0..n_pairs {
                    pairs.push((take::<isize>(lines)?, take::<isize>(lines)?));
                }
                parton_flavors.push(pairs);
            }
        } else {
            unreachable!();
        }
    } else {
        parton_flavors = Vec::new();
    }
    tracing::info!(
        "Found PDF info for {} PDFs containing {} subprocesses",
        pdfs.len(),
        n_subproc
    );
    return Ok(PDFInfo {
        pdfs,
        n_pdf_dim,
        fragmentation_functions,
        n_ff_dim,
        n_subproc,
        pdf_flag_1,
        pdf_flag_2,
        pdf_flag_3,
        parton_flavors,
    });
}

#[inline]
#[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
fn read_grid<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    norm: Float,
    scale_dependence: usize,
    n_bins: usize,
    n_scale_dim: usize,
    n_subproc: usize,
    n_pdf_dim: usize,
    x1_nodes: &Vec<Vec<Float>>,
    x2_nodes: &Vec<Vec<Float>>,
) -> Result<Grid, ReadError> {
    if scale_dependence == 0 {
        tracing::info!("Reading fixed-type grid");
        let mut n_scale_var = Vec::with_capacity(n_scale_dim);
        let mut n_scale_node = Vec::with_capacity(n_scale_dim);
        for _ in 0..n_scale_dim {
            n_scale_var.push(take::<usize>(lines)?);
            n_scale_node.push(take::<usize>(lines)?);
        }
        let n_var_max = *n_scale_var.iter().max().unwrap();
        let mut scale_fac = Array2::zeros((n_scale_dim, n_var_max));
        for i in 0..n_scale_dim {
            for j in 0..n_scale_var[i] {
                scale_fac[[i, j]] = take::<Float>(lines)?;
            }
        }
        let n_node_max = *n_scale_node.iter().max().unwrap();
        let mut scale_node = Array4::zeros((n_bins, n_scale_dim, n_var_max, n_node_max));
        for i in 0..n_bins {
            for j in 0..n_scale_dim {
                for k in 0..n_scale_var[j] {
                    for l in 0..n_scale_node[j] {
                        scale_node[[i, j, k, l]] = take::<Float>(lines)?;
                    }
                }
            }
        }
        let nxmax_vec = match n_pdf_dim {
            0 => x1_nodes.iter().map(|l| l.len()).collect::<Vec<_>>(),
            1 => x1_nodes
                .iter()
                .map(|l| (l.len() * l.len() + l.len()) / 2)
                .collect::<Vec<_>>(),
            2 => x1_nodes
                .iter()
                .zip(x2_nodes.iter())
                .map(|(l1, l2)| l1.len() * l2.len())
                .collect::<Vec<_>>(),
            _ => unreachable!(),
        };
        let nxmax = *nxmax_vec.iter().max().unwrap();
        let mut grid =
            Array6::zeros((n_bins, n_scale_dim, n_var_max, n_node_max, nxmax, n_subproc));
        for i in 0..n_bins {
            for j in 0..n_scale_dim {
                for k in 0..n_scale_var[j] {
                    for l in 0..n_scale_node[j] {
                        for m in 0..nxmax_vec[i] {
                            for n in 0..n_subproc {
                                grid[[i, j, k, l, m, n]] = take::<Float>(lines)?;
                            }
                        }
                    }
                }
            }
        }
        tracing::info!(
            "Successfully read grid of dimension `{:?}` containing {} entries",
            grid.raw_dim(),
            grid.len()
        );
        return Ok(Grid::Fixed {
            n_scale_var,
            n_scale_node,
            scale_fac,
            scale_node,
            grid,
        });
    } else if scale_dependence >= 3 {
        match scale_dependence {
            4 => tracing::info!("Reading flex-type grid with contributions `[Finite]` (LO)"),
            5 => tracing::info!("Reading flex-type grid with contributions `[Finite, R, F]` (NLO)"),
            6 => tracing::info!(
                "Reading flex-type grid with contributions `[Finite, R, F, RR]` (NNLO)"
            ),
            7 => tracing::info!(
                "Reading flex-type grid with contributions `[Finite, R, F, RR, FF, RF]` (NNLO)"
            ),
            _ => unreachable!(),
        }
        let _n_bins = take::<usize>(lines)?;
        let n_scale_node_1 = take::<usize>(lines)?;
        let mut scale_node_1 = Array2::zeros((n_bins, n_scale_node_1));
        for i in 0..n_bins {
            if i != 0 {
                let _ = take::<usize>(lines)?;
            }
            for j in 0..n_scale_node_1 {
                scale_node_1[[i, j]] = take::<Float>(lines)?;
            }
        }
        let _n_bins = take::<usize>(lines)?;
        let n_scale_node_2 = take::<usize>(lines)?;
        let mut scale_node_2 = Array2::zeros((n_bins, n_scale_node_2));
        for i in 0..n_bins {
            if i != 0 {
                let _ = take::<usize>(lines)?;
            }
            for j in 0..n_scale_node_2 {
                scale_node_2[[i, j]] = take::<Float>(lines)?;
            }
        }
        let nxmax_vec = match n_pdf_dim {
            0 => x1_nodes.iter().map(|l| l.len()).collect::<Vec<_>>(),
            1 => x1_nodes
                .iter()
                .map(|l| (l.len() * l.len() + l.len()) / 2)
                .collect::<Vec<_>>(),
            2 => x1_nodes
                .iter()
                .zip(x2_nodes.iter())
                .map(|(l1, l2)| l1.len() * l2.len())
                .collect::<Vec<_>>(),
            _ => unreachable!(),
        };
        let nxmax = *nxmax_vec.iter().max().unwrap();
        let grid_shape = (n_bins, nxmax, n_scale_node_1, n_scale_node_2, n_subproc);
        let mut grid = Array5::zeros(grid_shape);
        fill_grid(lines, grid.view_mut(), n_subproc, norm)?;
        let mut grid_f;
        let mut grid_r;
        let mut grid_rr;
        let mut grid_ff;
        let mut grid_rf;
        if scale_dependence >= 5 {
            grid_f = Some(Array5::zeros(grid_shape));
            grid_r = Some(Array5::zeros(grid_shape));
            fill_grid(lines, grid_f.as_mut().unwrap().view_mut(), n_subproc, norm)?;
            fill_grid(lines, grid_r.as_mut().unwrap().view_mut(), n_subproc, norm)?;
            if scale_dependence >= 6 {
                grid_rr = Some(Array5::zeros(grid_shape));
                fill_grid(lines, grid_rr.as_mut().unwrap().view_mut(), n_subproc, norm)?;
                if scale_dependence >= 7 {
                    grid_ff = Some(Array5::zeros(grid_shape));
                    grid_rf = Some(Array5::zeros(grid_shape));
                    fill_grid(lines, grid_ff.as_mut().unwrap().view_mut(), n_subproc, norm)?;
                    fill_grid(lines, grid_rf.as_mut().unwrap().view_mut(), n_subproc, norm)?;
                } else {
                    grid_ff = None;
                    grid_rf = None;
                }
            } else {
                grid_rr = None;
                grid_ff = None;
                grid_rf = None;
            }
        } else {
            grid_f = None;
            grid_r = None;
            grid_rr = None;
            grid_ff = None;
            grid_rf = None;
        }
        let _n_bins = take::<usize>(lines)?;
        let sigma_ref_mixed =
            Array2::from_shape_simple_fn((n_bins, n_subproc), || take::<Float>(lines).unwrap());
        let _n_bins = take::<usize>(lines)?;
        let sigma_ref_s1 =
            Array2::from_shape_simple_fn((n_bins, n_subproc), || take::<Float>(lines).unwrap());
        let _n_bins = take::<usize>(lines)?;
        let sigma_ref_s2 =
            Array2::from_shape_simple_fn((n_bins, n_subproc), || take::<Float>(lines).unwrap());
        tracing::info!(
            "Successfully read {} grids of dimensions `{:?}` containing {} entries each",
            match scale_dependence {
                4 => 1,
                5 => 3,
                6 => 4,
                7 => 6,
                _ => unreachable!(),
            },
            grid.raw_dim(),
            grid.len()
        );
        return Ok(Grid::Flex {
            scale_node_1,
            scale_node_2,
            grid,
            grid_f,
            grid_r,
            grid_rr,
            grid_ff,
            grid_rf,
            sigma_ref_mixed,
            sigma_ref_s1,
            sigma_ref_s2,
        });
    } else {
        unreachable!();
    }
}

#[inline]
#[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
fn fill_grid<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    mut grid: ArrayViewMut5<Float>,
    n_subproc: usize,
    norm: Float,
) -> Result<(), ReadError> {
    let n_bins = take::<usize>(lines)?;
    for i in 0..n_bins {
        let nxmax = take::<usize>(lines)?;
        for j in 0..nxmax {
            let n_scale_node_1 = take::<usize>(lines)?;
            for k in 0..n_scale_node_1 {
                let n_scale_node_2 = take::<usize>(lines)?;
                for l in 0..n_scale_node_2 {
                    for m in 0..n_subproc {
                        grid[[i, j, k, l, m]] = norm * take::<Float>(lines)?;
                    }
                }
            }
        }
    }
    return Ok(());
}

#[inline]
fn take<'a, T: FromStr>(lines: &mut impl Iterator<Item = &'a str>) -> Result<T, ReadError>
where
    ReadError: From<<T as FromStr>::Err>,
    <T as FromStr>::Err: std::fmt::Debug,
{
    return Ok(lines
        .next()
        .ok_or(ReadError::UnexpectedEOFError(format!(
            "`{}`",
            type_name::<T>()
        )))?
        .parse::<T>()?);
}

#[inline]
fn take_multiple<'a, T: FromStr>(
    lines: &mut impl Iterator<Item = &'a str>,
    n: Option<usize>,
) -> Result<Vec<T>, ReadError>
where
    ReadError: From<<T as FromStr>::Err>,
    <T as FromStr>::Err: std::fmt::Debug,
{
    let n = n.unwrap_or_else(|| take::<usize>(lines).unwrap());
    let res = lines
        .by_ref()
        .take(n)
        .map(|s| s.parse::<T>())
        .collect::<Result<Vec<_>, _>>();
    match res {
        Ok(x) => {
            return if x.len() == n {
                Ok(x)
            } else {
                Err(ReadError::UnexpectedEOFError(format!(
                    "`{}` times `{}`, found `{}`",
                    n,
                    type_name::<T>(),
                    x.len()
                )))
            };
        }
        Err(e) => Err(ReadError::from(e)),
    }
}

#[inline]
fn take_nested_vec<'a, T: FromStr>(
    lines: &mut impl Iterator<Item = &'a str>,
    n: Option<usize>,
) -> Result<Vec<Vec<T>>, ReadError>
where
    ReadError: From<<T as FromStr>::Err>,
    <T as FromStr>::Err: std::fmt::Debug,
{
    let n = n.unwrap_or_else(|| take::<usize>(lines).unwrap());
    let mut res = Vec::with_capacity(n);
    for _ in 0..n {
        let m = take::<usize>(lines)?;
        res.push(take_multiple::<T>(lines, Some(m))?);
    }
    return Ok(res);
}
