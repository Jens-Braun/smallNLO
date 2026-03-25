use ndarray::Array5;

use crate::Float;
use crate::error::WriteError;
use crate::table::{BinInfo, Block, BlockData, FastNLOFile, Grid, Metadata, PDFInfo};
use std::fmt::Write;

const SEP: &str = "1234567890";

pub(crate) fn write_fastnlo(w: &mut impl Write, tab: &FastNLOFile) -> Result<(), WriteError> {
    w_literal(w, SEP)?;
    write_metadata(w, &tab.metadata)?;
    write_bins(w, &tab.bin_info)?;
    w_literal(w, 0)?; // INormFlag, assume always zero
    for b in tab.blocks.iter() {
        write_block(w, b)?;
    }
    w_literal(w, SEP)?;
    w_literal(w, SEP)?;
    return Ok(());
}

fn write_metadata(w: &mut impl Write, metadata: &Metadata) -> Result<(), WriteError> {
    w_literal(w, metadata.table_version)?;
    w_literal(w, &metadata.scenario_name)?;
    w_literal(w, metadata.n_contrib)?;
    w_literal(w, metadata.n_mult)?;
    w_literal(w, metadata.n_data)?;
    w_literal(w, 0)?; // NuserString
    w_literal(w, 0)?; // NuserInt
    w_literal(w, 0)?; // NuserFloat
    w_literal(w, 0)?; // Imachine

    /*w_literal(w, metadata.n_user)?;
    for _ in 0..metadata.n_user {
        todo!();
    }*/
    w_literal(w, SEP)?;
    w_literal(w, metadata.unit)?;
    w_slice(w, &metadata.description)?;
    w_literal(w, metadata.cms_energy)?;
    w_literal(w, metadata.alphas_ord)?;
    return Ok(());
}

fn write_bins(w: &mut impl Write, bin_info: &BinInfo) -> Result<(), WriteError> {
    w_literal(w, bin_info.bins.len())?;
    w_literal(w, bin_info.bins[0].values.len())?;
    w_iter(w, &bin_info.dim_labels)?;
    w_iter(w, &bin_info.diff_bin)?;
    for bin in bin_info.bins.iter() {
        for v in bin.values.iter() {
            match v {
                super::BinPosition::Central(x) => w_literal(w, x)?,
                super::BinPosition::Boundaries { low, high } => {
                    w_literal(w, low)?;
                    w_literal(w, high)?;
                }
            }
        }
    }
    w_iter(w, bin_info.bins.iter().map(|b| b.size))?;
    return Ok(());
}

fn write_block(w: &mut impl Write, b: &Block) -> Result<(), WriteError> {
    w_literal(w, SEP)?;
    w_literal(w, b.unit)?;
    w_literal(w, b.data_block as i32)?;
    w_literal(w, b.mult_block as i32)?;
    w_literal(w, b.contribution_type)?;
    w_literal(w, b.contribution_order)?;
    w_literal(w, b.scale_format)?;
    w_slice(w, &b.description)?;
    w_slice(w, &b.code_description)?;
    match &b.data {
        BlockData::DataBlock { .. } => write_data_block(w, &b.data)?,
        BlockData::MultBlock { .. } => write_mult_block(w, &b.data)?,
        BlockData::TheoryBlock { grid, .. } => {
            if let Grid::Flex {
                grid_f,
                grid_r,
                grid_rr,
                grid_ff,
                grid_rf,
                ..
            } = grid
            {
                if (b.scale_format >= 5 && (grid_f.is_none() || grid_r.is_none()))
                    || (b.scale_format >= 6 && grid_rr.is_none())
                    || (b.scale_format >= 7 && (grid_ff.is_none() || grid_rf.is_none()))
                {
                    tracing::warn!(
                        "Exporting stripped grids to FastNLO is unsupported and will probably result in errors."
                    )
                }
            }
            write_theory_block(w, &b.data)?;
        }
    }
    w_literal(w, b.coeff_info_flags_1.len())?;
    for i in 0..b.coeff_info_flags_1.len() {
        w_literal(w, b.coeff_info_flags_1[i])?;
        w_literal(w, b.coeff_info_flags_2[i])?;
        w_slice(w, &b.coeff_block_description[i])?;
        w_slice(w, &b.coeff_block_content[i])?;
    }
    return Ok(());
}

fn write_data_block(w: &mut impl Write, b: &BlockData) -> Result<(), WriteError> {
    match b {
        BlockData::DataBlock {
            uncorr_sources,
            corr_sources,
            centers,
            values,
            uncorr_low,
            uncorr_high,
            corr_low,
            corr_high,
            corr_matrix,
        } => {
            w_slice(w, uncorr_sources)?;
            w_slice(w, corr_sources)?;
            for i in 0..centers.len() {
                w_literal(w, centers[i])?;
                w_literal(w, values[i])?;
                for j in 0..uncorr_sources.len() {
                    w_literal(w, uncorr_low[[i, j]])?;
                    w_literal(w, uncorr_high[[i, j]])?;
                }
                for j in 0..corr_sources.len() {
                    w_literal(w, corr_low[[i, j]])?;
                    w_literal(w, corr_high[[i, j]])?;
                }
            }
            w_literal(w, corr_matrix.dim().0)?;
            w_iter(w, corr_matrix.iter())?;
        }
        _ => unreachable!(),
    }
    return Ok(());
}

fn write_mult_block(w: &mut impl Write, b: &BlockData) -> Result<(), WriteError> {
    match b {
        BlockData::MultBlock {
            uncorr_sources,
            corr_sources,
            centers,
            values,
            uncorr_low,
            uncorr_high,
            corr_low,
            corr_high,
        } => {
            w_slice(w, uncorr_sources)?;
            w_slice(w, corr_sources)?;
            for i in 0..centers.len() {
                w_literal(w, centers[i])?;
                w_literal(w, values[i])?;
                for j in 0..uncorr_sources.len() {
                    w_literal(w, uncorr_low[[i, j]])?;
                    w_literal(w, uncorr_high[[i, j]])?;
                }
                for j in 0..corr_sources.len() {
                    w_literal(w, corr_low[[i, j]])?;
                    w_literal(w, corr_high[[i, j]])?;
                }
            }
        }
        _ => unreachable!(),
    }
    return Ok(());
}

fn write_theory_block(w: &mut impl Write, b: &BlockData) -> Result<(), WriteError> {
    match b {
        BlockData::TheoryBlock {
            reference_table,
            i_scale_dependence,
            n_events,
            weight_info,
            alphas_power,
            pdf_info,
            x1_nodes,
            x2_nodes,
            z_nodes,
            scale_dimension,
            scale_description,
            grid,
        } => {
            w_literal(w, *reference_table as i32)?;
            w_literal(w, i_scale_dependence)?;
            w_literal(w, n_events)?;
            if let Some(weight_info) = weight_info {
                w_literal(w, weight_info.n_events)?;
                w_literal(w, weight_info.norm)?;
                if *n_events <= -2 {
                    w_literal(w, weight_info.n_tables)?;
                }
                w_literal(w, weight_info.n_entries)?;
                w_literal(w, weight_info.sum_weights_sq)?;
                w_literal(w, weight_info.sum_sig_sq)?;
                w_literal(w, weight_info.sum_sig)?;
                w_nested_vec(w, &weight_info.weight_sq_obs)?;
                w_nested_vec(w, &weight_info.sig_sq_obs)?;
                w_nested_vec(w, &weight_info.sig_obs)?;
                w_nested_vec(w, &weight_info.n_events_obs)?;
            }
            w_literal(w, alphas_power)?;
            write_pdf_info(w, pdf_info)?;
            for v in x1_nodes.iter() {
                w_slice(w, v)?;
            }
            for v in x2_nodes.iter() {
                w_slice(w, v)?;
            }
            for v in z_nodes.iter() {
                w_slice(w, v)?;
            }
            w_literal(w, scale_dimension.len())?;
            w_literal(w, scale_description.len())?;
            w_iter(w, scale_dimension)?;
            for v in scale_description.iter() {
                w_slice(w, v)?;
            }
            write_grid(
                w,
                grid,
                if let Some(wgt) = weight_info {
                    wgt.norm as Float
                } else {
                    1.
                },
            )?;
        }
        _ => unreachable!(),
    }
    return Ok(());
}

fn write_pdf_info(w: &mut impl Write, pdf: &PDFInfo) -> Result<(), WriteError> {
    w_slice(w, &pdf.pdfs)?;
    w_literal(w, pdf.n_pdf_dim)?;
    w_slice(w, &pdf.fragmentation_functions)?;
    w_literal(w, pdf.n_ff_dim)?;
    w_literal(w, pdf.n_subproc)?;
    w_literal(w, pdf.pdf_flag_1)?;
    w_literal(w, pdf.pdf_flag_2)?;
    w_literal(w, pdf.pdf_flag_3)?;
    if pdf.pdf_flag_2 == 0 {
        w_literal(w, 0)?; // coeff_format, always 0
        for sp in pdf.parton_flavors.iter() {
            w_literal(w, sp.len())?;
            for pair in sp.iter() {
                w_literal(w, pair.0)?;
                w_literal(w, pair.1)?;
            }
        }
    }
    return Ok(());
}

// TODO: This implementation currently assumes all grids to be contiguous, i.e. have the same number of scale
//       nodes in all bins.
fn write_grid(w: &mut impl Write, g: &Grid, norm: Float) -> Result<(), WriteError> {
    match g {
        Grid::Fixed {
            n_scale_var,
            n_scale_node,
            scale_fac,
            scale_node,
            grid,
        } => {
            for i in 0..n_scale_var.len() {
                w_literal(w, n_scale_var[i])?;
                w_literal(w, n_scale_node[i])?;
            }
            w_iter(w, scale_fac.iter())?;
            w_iter(w, scale_node.iter())?;
            w_iter(w, grid.iter())?;
        }
        Grid::Flex {
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
        } => {
            w_literal(w, scale_node_1.dim().0)?;
            for v in scale_node_1.rows() {
                w_slice(w, v.as_slice().unwrap())?;
            }
            w_literal(w, scale_node_2.dim().0)?;
            for v in scale_node_2.rows() {
                w_slice(w, v.as_slice().unwrap())?;
            }
            write_grid_array(w, grid, norm)?;
            if let Some(grid_f) = grid_f {
                write_grid_array(w, grid_f, norm)?;
            }
            if let Some(grid_r) = grid_r {
                write_grid_array(w, grid_r, norm)?;
            }
            if let Some(grid_rr) = grid_rr {
                write_grid_array(w, grid_rr, norm)?;
            }
            if let Some(grid_ff) = grid_ff {
                write_grid_array(w, grid_ff, norm)?;
            }
            if let Some(grid_rf) = grid_rf {
                write_grid_array(w, grid_rf, norm)?;
            }
            w_literal(w, sigma_ref_mixed.dim().0)?;
            for v in sigma_ref_mixed.rows() {
                w_iter(w, v.as_slice().unwrap())?;
            }
            w_literal(w, sigma_ref_s1.dim().0)?;
            for v in sigma_ref_s1.rows() {
                w_iter(w, v.as_slice().unwrap())?;
            }
            w_literal(w, sigma_ref_s2.dim().0)?;
            for v in sigma_ref_s2.rows() {
                w_iter(w, v.as_slice().unwrap())?;
            }
        }
    }
    return Ok(());
}

fn write_grid_array(w: &mut impl Write, grid: &Array5<Float>, norm: Float) -> Result<(), WriteError> {
    let (n_bins, nxmax, n_scale_node_1, n_scale_node_2, n_subproc) = grid.dim();
    let inv_norm = 1. / norm;
    w_literal(w, n_bins)?;
    for i in 0..n_bins {
        w_literal(w, nxmax)?;
        for j in 0..nxmax {
            w_literal(w, n_scale_node_1)?;
            for k in 0..n_scale_node_1 {
                w_literal(w, n_scale_node_2)?;
                for l in 0..n_scale_node_2 {
                    for m in 0..n_subproc {
                        w_literal(w, inv_norm * grid[[i, j, k, l, m]])?;
                    }
                }
            }
        }
    }
    return Ok(());
}

#[inline]
fn w_literal<T: std::fmt::Display>(w: &mut impl Write, x: T) -> Result<(), WriteError> {
    writeln!(w, "{}", x)?;
    return Ok(());
}

#[inline]
fn w_slice<T: std::fmt::Display>(w: &mut impl Write, x: &[T]) -> Result<(), WriteError> {
    w_literal(w, x.len())?;
    for y in x {
        w_literal(w, y)?;
    }
    return Ok(());
}

#[inline]
fn w_iter<T: std::fmt::Display>(w: &mut impl Write, x: impl IntoIterator<Item = T>) -> Result<(), WriteError> {
    for y in x {
        w_literal(w, y)?;
    }
    return Ok(());
}

#[inline]
fn w_nested_vec<T: std::fmt::Display>(w: &mut impl Write, x: &Vec<Vec<T>>) -> Result<(), WriteError> {
    w_literal(w, x.len())?;
    for y in x {
        w_slice(w, y)?;
    }
    return Ok(());
}
