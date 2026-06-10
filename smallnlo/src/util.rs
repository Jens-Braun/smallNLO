use crate::Float;
use crate::table::{Block, BlockData, FastNLOFile, Grid, PDFInfo};
use hoppet::Hoppet;
use ndarray::prelude::*;
use std::sync::Arc;

pub(crate) const FRAC_1_TWOPI: Float = 0.15915494309189533576888;
pub(crate) const BETA_0: Float = 23. / 6.;
pub(crate) const BETA_1: Float = 29. / 3.;

pub(crate) fn conv_xfx(
    pdf_info: &PDFInfo,
    hp: &Hoppet,
    x1: Float,
    x2: Float,
    muf: Float,
    iloop1: i32,
    iloop2: i32,
    res: &mut [Float],
) {
    match pdf_info.pdfs.len() {
        0 => (),
        1 => todo!(),
        2 => {
            let mut buf1 = [0.; 13];
            if iloop1 == 0 {
                hp.eval(x1 as f64, muf as f64, &mut buf1);
            } else {
                hp.eval_split(x1 as f64, muf as f64, iloop1, &mut buf1);
            }
            let mut buf2 = [0.; 13];
            if iloop2 == 0 {
                hp.eval(x2 as f64, muf as f64, &mut buf2);
            } else {
                hp.eval_split(x2 as f64, muf as f64, iloop2, &mut buf2);
            }
            for k in 0..pdf_info.n_subproc {
                res[k] = pdf_info.parton_flavors[k]
                    .iter()
                    .map(|(i, j)| buf1[(*i + 6) as usize] * buf2[(*j + 6) as usize])
                    .sum::<f64>() as Float
            }
        }
        _ => unreachable!(),
    }
}

impl FastNLOFile {
    pub(crate) fn build_flex_pdf_grid(
        &self,
        b: &Block,
        hp: &Hoppet,
        mu: &Box<Arc<dyn Fn(Float, Float) -> Float + Send + Sync>>,
        iloop1: i32,
        iloop2: i32,
    ) -> Array5<Float> {
        match &b.data {
            BlockData::TheoryBlock {
                x1_nodes,
                x2_nodes,
                grid,
                pdf_info,
                ..
            } => match grid {
                Grid::Flex {
                    scale_node_1,
                    scale_node_2,
                    grid,
                    ..
                } => {
                    let mut pdf = Array5::zeros(grid.raw_dim());
                    for i in 0..grid.dim().0 {
                        let nxmax = match pdf_info.n_pdf_dim {
                            0 => x1_nodes[i].len(),
                            1 => x1_nodes[i].len() * (x1_nodes.len() + 1) / 2,
                            2 => x1_nodes[i].len() * x2_nodes[i].len(),
                            _ => unreachable!(),
                        };
                        for j in 0..scale_node_1.shape()[1] {
                            for k in 0..scale_node_2.shape()[1] {
                                let muf = (mu)(scale_node_1[[i, j]], scale_node_2[[i, k]]);
                                for x in 0..nxmax {
                                    crate::util::conv_xfx(
                                        pdf_info,
                                        &hp,
                                        x1_nodes[i][x % x1_nodes[i].len()],
                                        x2_nodes[i][x / x1_nodes[i].len()],
                                        muf,
                                        iloop1,
                                        iloop2,
                                        pdf.slice_mut(s![i, x, j, k, ..]).as_slice_mut().unwrap(),
                                    );
                                }
                            }
                        }
                    }
                    return pdf;
                }
                _ => unreachable!(),
            },
            _ => todo!(),
        }
    }

    pub(crate) fn build_fix_pdf_grid(&self, b: &Block, hp: &Hoppet) -> Array4<Float> {
        match &b.data {
            BlockData::TheoryBlock {
                x1_nodes,
                x2_nodes,
                grid,
                pdf_info,
                ..
            } => match grid {
                Grid::Fixed { grid, scale_node, .. } => {
                    let mut pdf = Array4::zeros((grid.dim().0, grid.dim().2, grid.dim().3, grid.dim().4));
                    for i in 0..grid.dim().0 {
                        let nxmax = match pdf_info.n_pdf_dim {
                            0 => x1_nodes[i].len(),
                            1 => x1_nodes[i].len() * (x1_nodes.len() + 1) / 2,
                            2 => x1_nodes[i].len() * x2_nodes[i].len(),
                            _ => unreachable!(),
                        };
                        for j in 0..scale_node.shape()[3] {
                            let muf = scale_node[[i, 0, 0, j]];
                            for k in 0..nxmax {
                                crate::util::conv_xfx(
                                    pdf_info,
                                    &hp,
                                    x1_nodes[i][k % x1_nodes[i].len()],
                                    x2_nodes[i][k / x1_nodes[i].len()],
                                    muf,
                                    0,
                                    0,
                                    pdf.slice_mut(s![i, j, k, ..]).as_slice_mut().unwrap(),
                                );
                            }
                        }
                    }
                    return pdf;
                }
                _ => unreachable!(),
            },
            _ => todo!(),
        }
    }

    /// Search for the block containing the data for which the order of ɑₛ is ref_order - n
    pub(crate) fn lower_order_grid(&self, ref_pdf: &PDFInfo, ref_order: usize, n: usize) -> LowerOrderGrid<'_> {
        for b in self.blocks.iter() {
            match &b.data {
                BlockData::DataBlock { .. } => (),
                BlockData::MultBlock { .. } => (),
                BlockData::TheoryBlock {
                    alphas_power,
                    grid,
                    weight_info,
                    pdf_info,
                    ..
                } => match grid {
                    Grid::Fixed { .. } => todo!(),
                    Grid::Flex { grid, .. } => {
                        if *alphas_power + n == ref_order {
                            let map = pdf_info
                                .parton_flavors
                                .iter()
                                .map(|subprocess| {
                                    ref_pdf
                                        .parton_flavors
                                        .iter()
                                        .position(|ref_proc| *subprocess == *ref_proc)
                                        .expect("Subprocess of lower order grid not present in higher order file")
                                })
                                .collect();
                            return LowerOrderGrid {
                                grid,
                                norm: weight_info.as_ref().unwrap().norm as Float,
                                subprocess_map: map,
                            };
                        }
                    }
                },
            }
        }
        panic!(
            "Unable to reconstruct scale dependence, no lower order grid present in table (required order in αₛ: {})",
            ref_order - n
        );
    }
}

pub(crate) struct LowerOrderGrid<'a> {
    pub(crate) grid: &'a Array5<Float>,
    pub(crate) norm: Float,
    pub(crate) subprocess_map: Vec<usize>,
}
