use crate::{
    Float,
    table::{Block, BlockData, FastNLOFile, Grid, PDFInfo},
    util::{BETA_0, BETA_1, FRAC_1_TWOPI},
};
use hoppet::{Hoppet, HoppetConfig, HoppetError};
use ndarray::{Zip, prelude::*};
use std::{f64, sync::Arc};

pub struct FastNLOEvalutator<'a, 'b> {
    file: &'a FastNLOFile,
    pdf: &'b str,
    mu_r_function: Box<Arc<dyn Fn(Float, Float) -> Float + Send + Sync>>,
    mu_f_function: Box<Arc<dyn Fn(Float, Float) -> Float + Send + Sync>>,
    max_order: Option<usize>,
}

impl<'a, 'b> FastNLOEvalutator<'a, 'b> {
    pub fn new(
        file: &'a FastNLOFile,
        pdf: &'b str,
        mu_r: Option<Box<Arc<dyn Fn(Float, Float) -> Float + Send + Sync>>>,
        mu_f: Option<Box<Arc<dyn Fn(Float, Float) -> Float + Send + Sync>>>,
    ) -> FastNLOEvalutator<'a, 'b> {
        return FastNLOEvalutator {
            file,
            pdf,
            mu_r_function: mu_r.unwrap_or(Box::new(Arc::new(|s1, _s2| s1))),
            mu_f_function: mu_f.unwrap_or(Box::new(Arc::new(|s1, _s2| s1))),
            max_order: None,
        };
    }
    pub fn max_power(&mut self, power: Option<usize>) -> &mut Self {
        self.max_order = power;
        return self;
    }
}

impl FastNLOEvalutator<'_, '_> {
    pub fn cross_sections(&self) -> Result<Array1<Float>, HoppetError> {
        let hp = HoppetConfig::init_from_lhapdf(self.pdf, 0)?;
        let mut xsec = Array1::zeros(self.file.bins.len());
        for block in self.file.blocks.iter() {
            match &block.data {
                BlockData::DataBlock { .. } => todo!(),
                BlockData::MultBlock { .. } => todo!(),
                BlockData::TheoryBlock {
                    scale_dependence,
                    alphas_power,
                    grid,
                    weight_info,
                    pdf_info,
                    ..
                } => {
                    if let Some(power) = self.max_order
                        && power < *alphas_power - self.file.metadata.alphas_ord
                    {
                        continue;
                    }
                    match grid {
                        Grid::Fixed { .. } => todo!(),
                        Grid::Flex {
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
                            let unit = 10_f32.powi((self.file.metadata.unit as i32) - (block.unit as i32));
                            let normalization = unit / weight_info.as_ref().unwrap().norm;
                            let p = *alphas_power as Float;
                            for (i, xs) in xsec.iter_mut().enumerate() {
                                let alphas =
                                    Array2::from_shape_fn((scale_node_1.dim().1, scale_node_2.dim().1), |(j, k)| {
                                        (hp.alphaS(
                                            (self.mu_r_function)(scale_node_1[[i, j]], scale_node_2[[i, k]]) as f64
                                        ) as Float
                                            * FRAC_1_TWOPI)
                                            .powi(*alphas_power as i32)
                                    });
                                let pdf = self.build_pdf_grid(block, &hp, i, 0, 0);

                                *xs += normalization
                                    * Zip::indexed(grid.index_axis(Axis(0), i))
                                        .and(&pdf)
                                        .fold(0., |acc, (_, j, k, _), sigma, pdflc| {
                                            acc + sigma * alphas[[j, k]] * pdflc
                                        });
                                if *scale_dependence >= 5 {
                                    let lr2 = Array2::from_shape_fn(
                                        (scale_node_1.dim().1, scale_node_2.dim().1),
                                        |(j, k)| {
                                            (self.mu_r_function)(scale_node_1[[i, j]], scale_node_2[[i, k]]).ln() * 2.
                                        },
                                    );
                                    let lf2 = Array2::from_shape_fn(
                                        (scale_node_1.dim().1, scale_node_2.dim().1),
                                        |(j, k)| {
                                            (self.mu_f_function)(scale_node_1[[i, j]], scale_node_2[[i, k]]).ln() * 2.
                                        },
                                    );
                                    if let Some(grid_f) = grid_f
                                        && let Some(grid_r) = grid_r
                                    {
                                        *xs += normalization
                                            * Zip::indexed(grid_f.index_axis(Axis(0), i)).and(&pdf).fold(
                                                0.,
                                                |acc, (_, j, k, _), sigma, pdflc| {
                                                    acc + sigma * alphas[[j, k]] * pdflc * lf2[[j, k]]
                                                },
                                            );
                                        *xs += normalization
                                            * Zip::indexed(grid_r.index_axis(Axis(0), i)).and(&pdf).fold(
                                                0.,
                                                |acc, (_, j, k, _), sigma, pdflc| {
                                                    acc + sigma * alphas[[j, k]] * pdflc * lr2[[j, k]]
                                                },
                                            );
                                    } else {
                                        let pdf_0_1 = self.build_pdf_grid(block, &hp, i, 0, 1);
                                        let pdf_1_0 = self.build_pdf_grid(block, &hp, i, 1, 0);
                                        let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 1);
                                        *xs += unit / lo_grid.norm
                                            * Zip::indexed(lo_grid.grid.index_axis(Axis(0), i)).fold(
                                                0.,
                                                |acc, (x, j, k, n), sigma_lo| {
                                                    let m = lo_grid.subprocess_map[n];
                                                    acc + sigma_lo
                                                        * alphas[[j, k]]
                                                        * ((p - 1.) * BETA_0 * pdf[[x, j, k, m]] * lr2[[j, k]]
                                                            - lf2[[j, k]]
                                                                * (pdf_0_1[[x, j, k, m]] + pdf_1_0[[x, j, k, m]]))
                                                },
                                            );
                                    }
                                    if *scale_dependence >= 6 {
                                        if let Some(grid_rr) = grid_rr {
                                            *xs += normalization
                                                * Zip::indexed(grid_rr.index_axis(Axis(0), i)).and(&pdf).fold(
                                                    0.,
                                                    |acc, (_, j, k, _), sigma, pdflc| {
                                                        acc + sigma * alphas[[j, k]] * pdflc * lr2[[j, k]] * lr2[[j, k]]
                                                    },
                                                );
                                        } else {
                                            let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 2);
                                            *xs += unit / lo_grid.norm
                                                * Zip::indexed(lo_grid.grid.index_axis(Axis(0), i)).fold(
                                                    0.,
                                                    |acc, (x, j, k, n), sigma_lo| {
                                                        acc + sigma_lo
                                                            * alphas[[j, k]]
                                                            * (((p - 1.) * (p - 2.)) as Float
                                                                * 0.5
                                                                * BETA_0
                                                                * BETA_0
                                                                * pdf[[x, j, k, lo_grid.subprocess_map[n]]]
                                                                * lr2[[j, k]]
                                                                * lr2[[j, k]])
                                                    },
                                                );
                                        }
                                    }
                                    if *scale_dependence >= 7 {
                                        if let Some(grid_rf) = grid_rf
                                            && let Some(grid_ff) = grid_ff
                                        {
                                            *xs += normalization
                                                * Zip::indexed(grid_rf.index_axis(Axis(0), i)).and(&pdf).fold(
                                                    0.,
                                                    |acc, (_, j, k, _), sigma, pdflc| {
                                                        acc + sigma * alphas[[j, k]] * pdflc * lr2[[j, k]] * lf2[[j, k]]
                                                    },
                                                );
                                            *xs += normalization
                                                * Zip::indexed(grid_ff.index_axis(Axis(0), i)).and(&pdf).fold(
                                                    0.,
                                                    |acc, (_, j, k, _), sigma, pdflc| {
                                                        acc + sigma * alphas[[j, k]] * pdflc * lf2[[j, k]] * lf2[[j, k]]
                                                    },
                                                );
                                        } else {
                                            let pdf_0_1 = self.build_pdf_grid(block, &hp, i, 0, 1);
                                            let pdf_1_0 = self.build_pdf_grid(block, &hp, i, 1, 0);
                                            let pdf_0_11 = self.build_pdf_grid(block, &hp, i, 0, 11);
                                            let pdf_11_0 = self.build_pdf_grid(block, &hp, i, 11, 0);
                                            let pdf_0_2 = self.build_pdf_grid(block, &hp, i, 0, 2);
                                            let pdf_2_0 = self.build_pdf_grid(block, &hp, i, 2, 0);
                                            let pdf_1_1 = self.build_pdf_grid(block, &hp, i, 1, 1);
                                            let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 2);
                                            *xs += unit / lo_grid.norm
                                                * Zip::indexed(lo_grid.grid.index_axis(Axis(0), i)).fold(
                                                    0.,
                                                    |acc, (x, j, k, n), sigma_lo| {
                                                        let m = lo_grid.subprocess_map[n];
                                                        acc + sigma_lo
                                                            * alphas[[j, k]]
                                                            * ((p - 2.) as Float
                                                                * BETA_1
                                                                * pdf[[x, j, k, m]]
                                                                * lr2[[j, k]]
                                                                - (pdf_2_0[[x, j, k, m]] + pdf_0_2[[x, j, k, m]])
                                                                    * lf2[[j, k]]
                                                                + (pdf_11_0[[x, j, k, m]] + pdf_0_11[[x, j, k, m]])
                                                                    * lf2[[j, k]]
                                                                    * lf2[[j, k]]
                                                                    * 0.5
                                                                + BETA_0
                                                                    * (0.5 * lf2[[j, k]] - (p - 1.) * lr2[[j, k]])
                                                                    * (pdf_0_1[[x, j, k, m]] + pdf_1_0[[x, j, k, m]])
                                                                    * lf2[[j, k]]
                                                                + pdf_1_1[[x, j, k, m]] * lf2[[j, k]] * lf2[[j, k]])
                                                    },
                                                );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        return Ok(xsec);
    }

    fn build_pdf_grid(&self, b: &Block, hp: &Hoppet, i: usize, iloop1: i32, iloop2: i32) -> Array4<Float> {
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
                    let nxmax = match pdf_info.n_pdf_dim {
                        0 => x1_nodes[i].len(),
                        1 => x1_nodes[i].len() * (x1_nodes.len() + 1) / 2,
                        2 => x1_nodes[i].len() * x2_nodes[i].len(),
                        _ => unreachable!(),
                    };
                    let mut pdf = Array4::zeros((grid.dim().1, grid.dim().2, grid.dim().3, grid.dim().4));
                    for j in 0..scale_node_1.shape()[1] {
                        for k in 0..scale_node_2.shape()[1] {
                            let muf = (self.mu_f_function)(scale_node_1[[i, j]], scale_node_2[[i, k]]);
                            for x in 0..nxmax {
                                conv_xfx(
                                    pdf_info,
                                    &hp,
                                    x1_nodes[i][x % x1_nodes[i].len()],
                                    x2_nodes[i][x / x1_nodes[i].len()],
                                    muf,
                                    iloop1,
                                    iloop2,
                                    pdf.slice_mut(s![x, j, k, ..]).as_slice_mut().unwrap(),
                                );
                            }
                        }
                    }
                    return pdf;
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    /// Search for the block containing the data for which the order of ɑₛ is ref_order - n
    fn lower_order_grid(&self, ref_pdf: &PDFInfo, ref_order: usize, n: usize) -> LowerOrderGrid<'_> {
        for b in self.file.blocks.iter() {
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
                                        .expect("Subprocess of lower order grid not present in higher order grid")
                                })
                                .collect();
                            return LowerOrderGrid {
                                grid,
                                norm: weight_info.as_ref().unwrap().norm,
                                subprocess_map: map,
                            };
                        }
                    }
                },
            }
        }
        unreachable!();
    }
}

struct LowerOrderGrid<'a> {
    grid: &'a Array5<Float>,
    norm: Float,
    subprocess_map: Vec<usize>,
}

fn conv_xfx(
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
