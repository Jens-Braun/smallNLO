use crate::{
    Float,
    table::{BlockData, FastNLOFile, Grid},
    util::{BETA_0, BETA_1, FRAC_1_TWOPI},
};
use hoppet::{HoppetConfig, HoppetError};
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
        let mut xsec = Array1::zeros(self.file.bin_info.bins.len());
        for block in self.file.blocks.iter() {
            match &block.data {
                BlockData::DataBlock { .. } => todo!(),
                BlockData::MultBlock { .. } => todo!(),
                BlockData::TheoryBlock {
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
                            let unit = (10.0 as Float).powi((self.file.metadata.unit as i32) - (block.unit as i32));
                            let normalization = unit / weight_info.as_ref().unwrap().norm as Float;
                            let p = *alphas_power as Float;
                            let n = *alphas_power - self.file.metadata.alphas_ord;
                            let pdf = self.file.build_pdf_grid(block, &hp, &self.mu_f_function, 0, 0);
                            let mut pdfc_0_1 = None;
                            let mut pdfc_1_0 = None;
                            let mut pdfc_0_2 = None;
                            let mut pdfc_2_0 = None;
                            let mut pdfc_0_11 = None;
                            let mut pdfc_11_0 = None;
                            let mut pdfc_1_1 = None;
                            for (i, xs) in xsec.iter_mut().enumerate() {
                                let alphas =
                                    Array2::from_shape_fn((scale_node_1.dim().1, scale_node_2.dim().1), |(j, k)| {
                                        (hp.alphaS(
                                            (self.mu_r_function)(scale_node_1[[i, j]], scale_node_2[[i, k]]) as f64
                                        ) as Float
                                            * FRAC_1_TWOPI)
                                            .powi(*alphas_power as i32)
                                    });

                                *xs += normalization
                                    * Zip::indexed(grid.index_axis(Axis(0), i))
                                        .and(pdf.index_axis(Axis(0), i))
                                        .fold(0., |acc, (_, j, k, _), sigma, pdflc| {
                                            acc + sigma * alphas[[j, k]] * pdflc
                                        });
                                if block.scale_format >= 5 {
                                    // Include R and F
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
                                    if let Some(grid_r) = grid_r {
                                        *xs += normalization
                                            * Zip::indexed(grid_r.index_axis(Axis(0), i))
                                                .and(&pdf.index_axis(Axis(0), i))
                                                .fold(0., |acc, (_, j, k, _), sigma, pdflc| {
                                                    acc + sigma * alphas[[j, k]] * pdflc * lr2[[j, k]]
                                                });
                                    } else {
                                        tracing::info!("R piece not in table, reconstructing");
                                        let grid_1 = self.file.lower_order_grid(pdf_info, *alphas_power, 1);
                                        *xs += unit / grid_1.norm
                                            * Zip::indexed(grid_1.grid.index_axis(Axis(0), i)).fold(
                                                0.,
                                                |acc, (x, j, k, n), sigma_1| {
                                                    let m = grid_1.subprocess_map[n];
                                                    acc + sigma_1
                                                        * alphas[[j, k]]
                                                        * (p - 1.)
                                                        * BETA_0
                                                        * pdf[[i, x, j, k, m]]
                                                        * lr2[[j, k]]
                                                },
                                            );
                                        if n == 2 {
                                            let grid_2 = self.file.lower_order_grid(pdf_info, *alphas_power, 2);
                                            *xs += unit / grid_2.norm
                                                * Zip::indexed(grid_2.grid.index_axis(Axis(0), i)).fold(
                                                    0.,
                                                    |acc, (x, j, k, n), sigma_2| {
                                                        let m = grid_2.subprocess_map[n];
                                                        acc + sigma_2
                                                            * alphas[[j, k]]
                                                            * (p - 2.)
                                                            * BETA_1
                                                            * pdf[[i, x, j, k, m]]
                                                            * lr2[[j, k]]
                                                    },
                                                );
                                        }
                                    }
                                    if let Some(grid_f) = grid_f {
                                        *xs += normalization
                                            * Zip::indexed(grid_f.index_axis(Axis(0), i))
                                                .and(&pdf.index_axis(Axis(0), i))
                                                .fold(0., |acc, (_, j, k, _), sigma, pdflc| {
                                                    acc + sigma * alphas[[j, k]] * pdflc * lf2[[j, k]]
                                                });
                                    } else {
                                        tracing::info!("F piece not in table, reconstructing");
                                        if pdfc_0_1.is_none() || pdfc_1_0.is_none() {
                                            pdfc_0_1 =
                                                Some(self.file.build_pdf_grid(block, &hp, &self.mu_f_function, 0, 1));
                                            pdfc_1_0 =
                                                Some(self.file.build_pdf_grid(block, &hp, &self.mu_f_function, 1, 0));
                                        }
                                        let pdf_0_1 = pdfc_0_1.as_ref().unwrap();
                                        let pdf_1_0 = pdfc_1_0.as_ref().unwrap();
                                        let grid_1 = self.file.lower_order_grid(pdf_info, *alphas_power, 1);
                                        // NLO grid -> LO pieces, NNLO -> NLO pieces
                                        *xs += unit / grid_1.norm
                                            * Zip::indexed(grid_1.grid.index_axis(Axis(0), i)).fold(
                                                0.,
                                                |acc, (x, j, k, n), sigma_1| {
                                                    let m = grid_1.subprocess_map[n];
                                                    acc - sigma_1
                                                        * alphas[[j, k]]
                                                        * lf2[[j, k]]
                                                        * (pdf_0_1[[i, x, j, k, m]] + pdf_1_0[[i, x, j, k, m]])
                                                },
                                            );
                                        if n == 2 {
                                            // NNLO grid -> add LO pieces
                                            if pdfc_0_2.is_none() || pdfc_2_0.is_none() {
                                                pdfc_0_2 = Some(self.file.build_pdf_grid(
                                                    block,
                                                    &hp,
                                                    &self.mu_f_function,
                                                    0,
                                                    2,
                                                ));
                                                pdfc_2_0 = Some(self.file.build_pdf_grid(
                                                    block,
                                                    &hp,
                                                    &self.mu_f_function,
                                                    2,
                                                    0,
                                                ));
                                            }
                                            let pdf_0_2 = pdfc_0_2.as_ref().unwrap();
                                            let pdf_2_0 = pdfc_2_0.as_ref().unwrap();
                                            let grid_2 = self.file.lower_order_grid(pdf_info, *alphas_power, 2);
                                            *xs += unit / grid_2.norm
                                                * Zip::indexed(grid_2.grid.index_axis(Axis(0), i)).fold(
                                                    0.,
                                                    |acc, (x, j, k, n), sigma_2| {
                                                        let m = grid_2.subprocess_map[n];
                                                        acc - sigma_2
                                                            * alphas[[j, k]]
                                                            * lf2[[j, k]]
                                                            * (pdf_0_2[[i, x, j, k, m]] + pdf_2_0[[i, x, j, k, m]])
                                                    },
                                                );
                                        }
                                    }
                                    if block.scale_format >= 6 {
                                        // Include RR
                                        if let Some(grid_rr) = grid_rr {
                                            *xs += normalization
                                                * Zip::indexed(grid_rr.index_axis(Axis(0), i))
                                                    .and(&pdf.index_axis(Axis(0), i))
                                                    .fold(0., |acc, (_, j, k, _), sigma, pdflc| {
                                                        acc + sigma * alphas[[j, k]] * pdflc * lr2[[j, k]] * lr2[[j, k]]
                                                    });
                                        } else {
                                            tracing::info!("RR piece not in table, reconstructing");
                                            let lo_grid = self.file.lower_order_grid(pdf_info, *alphas_power, 2);
                                            *xs += unit / lo_grid.norm
                                                * Zip::indexed(lo_grid.grid.index_axis(Axis(0), i)).fold(
                                                    0.,
                                                    |acc, (x, j, k, n), sigma_lo| {
                                                        acc + sigma_lo
                                                            * alphas[[j, k]]
                                                            * (((p - 1.) * (p - 2.))
                                                                * 0.5
                                                                * BETA_0
                                                                * BETA_0
                                                                * pdf[[i, x, j, k, lo_grid.subprocess_map[n]]]
                                                                * lr2[[j, k]]
                                                                * lr2[[j, k]])
                                                    },
                                                );
                                        }
                                    }
                                    if block.scale_format >= 7 {
                                        if let Some(grid_rf) = grid_rf
                                            && let Some(grid_ff) = grid_ff
                                        {
                                            *xs += normalization
                                                * Zip::indexed(grid_rf.index_axis(Axis(0), i))
                                                    .and(&pdf.index_axis(Axis(0), i))
                                                    .fold(0., |acc, (_, j, k, _), sigma, pdflc| {
                                                        acc + sigma * alphas[[j, k]] * pdflc * lr2[[j, k]] * lf2[[j, k]]
                                                    });
                                            *xs += normalization
                                                * Zip::indexed(grid_ff.index_axis(Axis(0), i))
                                                    .and(&pdf.index_axis(Axis(0), i))
                                                    .fold(0., |acc, (_, j, k, _), sigma, pdflc| {
                                                        acc + sigma * alphas[[j, k]] * pdflc * lf2[[j, k]] * lf2[[j, k]]
                                                    });
                                        } else {
                                            tracing::info!("FF or RF piece not in table, reconstructing");
                                            if pdfc_0_1.is_none() || pdfc_1_0.is_none() {
                                                pdfc_0_1 = Some(self.file.build_pdf_grid(
                                                    block,
                                                    &hp,
                                                    &self.mu_f_function,
                                                    0,
                                                    1,
                                                ));
                                                pdfc_1_0 = Some(self.file.build_pdf_grid(
                                                    block,
                                                    &hp,
                                                    &self.mu_f_function,
                                                    1,
                                                    0,
                                                ));
                                            }
                                            if pdfc_0_11.is_none() || pdfc_11_0.is_none() || pdfc_1_1.is_none() {
                                                pdfc_0_11 = Some(self.file.build_pdf_grid(
                                                    block,
                                                    &hp,
                                                    &self.mu_f_function,
                                                    0,
                                                    11,
                                                ));
                                                pdfc_11_0 = Some(self.file.build_pdf_grid(
                                                    block,
                                                    &hp,
                                                    &self.mu_f_function,
                                                    11,
                                                    0,
                                                ));
                                                pdfc_1_1 = Some(self.file.build_pdf_grid(
                                                    block,
                                                    &hp,
                                                    &self.mu_f_function,
                                                    1,
                                                    1,
                                                ));
                                            }
                                            let pdf_0_1 = pdfc_0_1.as_ref().unwrap();
                                            let pdf_1_0 = pdfc_1_0.as_ref().unwrap();
                                            let pdf_0_11 = pdfc_0_11.as_ref().unwrap();
                                            let pdf_11_0 = pdfc_11_0.as_ref().unwrap();
                                            let pdf_1_1 = pdfc_1_1.as_ref().unwrap();
                                            let lo_grid = self.file.lower_order_grid(pdf_info, *alphas_power, 2);
                                            *xs += unit / lo_grid.norm
                                                * Zip::indexed(lo_grid.grid.index_axis(Axis(0), i)).fold(
                                                    0.,
                                                    |acc, (x, j, k, n), sigma_lo| {
                                                        let m = lo_grid.subprocess_map[n];
                                                        acc + sigma_lo
                                                            * alphas[[j, k]]
                                                            * 0.5
                                                            * (pdf_11_0[[i, x, j, k, m]]
                                                                + pdf_0_11[[i, x, j, k, m]]
                                                                + BETA_0
                                                                    * (pdf_1_0[[i, x, j, k, m]]
                                                                        + pdf_0_1[[i, x, j, k, m]])
                                                                + 2. * pdf_1_1[[i, x, j, k, m]])
                                                            * lf2[[j, k]]
                                                            * lf2[[j, k]]
                                                    },
                                                );
                                            *xs += unit / lo_grid.norm
                                                * Zip::indexed(lo_grid.grid.index_axis(Axis(0), i)).fold(
                                                    0.,
                                                    |acc, (x, j, k, n), sigma_lo| {
                                                        let m = lo_grid.subprocess_map[n];
                                                        acc - sigma_lo
                                                            * alphas[[j, k]]
                                                            * (p - 1.)
                                                            * BETA_0
                                                            * (pdf_1_0[[i, x, j, k, m]] + pdf_0_1[[i, x, j, k, m]])
                                                            * lf2[[j, k]]
                                                            * lr2[[j, k]]
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
}
