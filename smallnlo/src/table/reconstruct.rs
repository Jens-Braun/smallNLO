use crate::{
    Float,
    table::{BlockData, FastNLOFile, Grid},
    util::{BETA_0, BETA_1},
};
use hoppet::{HoppetConfig, HoppetError};
use std::{collections::VecDeque, sync::Arc};

impl FastNLOFile {
    #[tracing::instrument(skip_all, level = tracing::Level::DEBUG)]
    pub fn reconstruct(
        &mut self,
        pdf: &str,
        mu_f: Box<Arc<dyn Fn(Float, Float) -> Float + Send + Sync>>,
    ) -> Result<(), HoppetError> {
        let hp = HoppetConfig::init_from_lhapdf(pdf, 0)?;
        let mut grids_r = VecDeque::with_capacity(self.bin_info.bins.len());
        let mut grids_f = VecDeque::with_capacity(self.bin_info.bins.len());
        let mut grids_rr = VecDeque::with_capacity(self.bin_info.bins.len());
        let mut grids_ff = VecDeque::with_capacity(self.bin_info.bins.len());
        let mut grids_rf = VecDeque::with_capacity(self.bin_info.bins.len());
        // The compiler does not allow mutating the grids during this loop, therefore produce all grids and afterwards assign them
        for block in self.blocks.iter() {
            match &block.data {
                BlockData::DataBlock { .. } => todo!(),
                BlockData::MultBlock { .. } => todo!(),
                BlockData::TheoryBlock {
                    alphas_power,
                    grid,
                    pdf_info,
                    weight_info,
                    ..
                } => match grid {
                    Grid::Fixed { .. } => todo!(),
                    Grid::Flex {
                        grid_f,
                        grid_r,
                        grid_rr,
                        grid_ff,
                        grid_rf,
                        ..
                    } => {
                        let p = *alphas_power as Float;
                        let n = *alphas_power - self.metadata.alphas_ord;
                        tracing::debug!("Reconstructing grids for block at order {alphas_power} with `n = {n}`");
                        if block.scale_format <= 4 {
                            // LO grid, nothing to do
                            continue;
                        }
                        let pdf = self.build_flex_pdf_grid(block, &hp, &mu_f, 0, 0);
                        let mut pdfc_0_1 = None;
                        let mut pdfc_1_0 = None;
                        let mut pdfc_0_2 = None;
                        let mut pdfc_2_0 = None;
                        let mut pdfc_0_11 = None;
                        let mut pdfc_11_0 = None;
                        let mut pdfc_1_1 = None;
                        if grid_r.is_none() {
                            tracing::debug!("Reconstructing R grid");
                            match n {
                                1 => {
                                    let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 1);
                                    let delta_lo_norm = weight_info.as_ref().unwrap().norm as Float / lo_grid.norm;
                                    grids_r.push_back((p - 1.) * BETA_0 * lo_grid.grid.clone() * delta_lo_norm);
                                }
                                2 => {
                                    let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 2);
                                    let delta_lo_norm = weight_info.as_ref().unwrap().norm as Float / lo_grid.norm;
                                    let nlo_grid = self.lower_order_grid(pdf_info, *alphas_power, 1);
                                    let delta_nlo_norm = weight_info.as_ref().unwrap().norm as Float / nlo_grid.norm;
                                    grids_r.push_back(
                                        (p - 1.) * BETA_0 * nlo_grid.grid.clone() * delta_nlo_norm
                                            + (p - 2.) * BETA_1 * lo_grid.grid * delta_lo_norm,
                                    );
                                }
                                _ => unreachable!(),
                            }
                        }
                        if grid_f.is_none() {
                            tracing::debug!("Reconstructing F grid");
                            if pdfc_0_1.is_none() || pdfc_1_0.is_none() {
                                pdfc_0_1 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 0, 1));
                                pdfc_1_0 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 1, 0));
                            }
                            let pdf_0_1 = pdfc_0_1.as_ref().unwrap();
                            let pdf_1_0 = pdfc_1_0.as_ref().unwrap();
                            match n {
                                1 => {
                                    let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 1);
                                    let delta_lo_norm = weight_info.as_ref().unwrap().norm as Float / lo_grid.norm;
                                    grids_f
                                        .push_back(-(pdf_0_1 + pdf_1_0) / &pdf * lo_grid.grid.clone() * delta_lo_norm);
                                }
                                2 => {
                                    if pdfc_0_2.is_none() || pdfc_2_0.is_none() {
                                        pdfc_0_2 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 0, 2));
                                        pdfc_2_0 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 2, 0));
                                    }
                                    let pdf_0_2 = pdfc_0_2.as_ref().unwrap();
                                    let pdf_2_0 = pdfc_2_0.as_ref().unwrap();
                                    let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 2);
                                    let delta_lo_norm = weight_info.as_ref().unwrap().norm as Float / lo_grid.norm;
                                    let nlo_grid = self.lower_order_grid(pdf_info, *alphas_power, 1);
                                    let delta_nlo_norm = weight_info.as_ref().unwrap().norm as Float / nlo_grid.norm;
                                    grids_f.push_back(
                                        -((pdf_0_1 + pdf_1_0) * nlo_grid.grid.clone() * delta_nlo_norm
                                            + (pdf_0_2 + pdf_2_0) * lo_grid.grid * delta_lo_norm)
                                            / &pdf,
                                    );
                                }
                                _ => unreachable!(),
                            }
                        }
                        if block.scale_format <= 5 {
                            // NLO grid, nothing more to do
                            continue;
                        }
                        if grid_rr.is_none() {
                            tracing::debug!("Reconstructing RR grid");
                            let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 2);
                            let delta_lo_norm = weight_info.as_ref().unwrap().norm as Float / lo_grid.norm;
                            grids_rr.push_back(
                                0.5 * (p - 1.) * (p - 2.) * BETA_0 * BETA_0 * lo_grid.grid.clone() * delta_lo_norm,
                            );
                        }
                        if block.scale_format <= 6 {
                            // NNLO grid with only RR, nothing more to do
                            continue;
                        }
                        if grid_ff.is_none() {
                            tracing::debug!("Reconstructing FF grid");
                            if pdfc_0_1.is_none() || pdfc_1_0.is_none() {
                                pdfc_0_1 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 0, 1));
                                pdfc_1_0 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 1, 0));
                            }
                            if pdfc_0_11.is_none() || pdfc_11_0.is_none() || pdfc_1_1.is_none() {
                                pdfc_0_11 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 0, 11));
                                pdfc_11_0 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 11, 0));
                                pdfc_1_1 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 1, 1));
                            }
                            let pdf_0_1 = pdfc_0_1.as_ref().unwrap();
                            let pdf_1_0 = pdfc_1_0.as_ref().unwrap();
                            let pdf_0_11 = pdfc_0_11.as_ref().unwrap();
                            let pdf_11_0 = pdfc_11_0.as_ref().unwrap();
                            let pdf_1_1 = pdfc_1_1.as_ref().unwrap();
                            let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 2);
                            let delta_lo_norm = weight_info.as_ref().unwrap().norm as Float / lo_grid.norm;
                            grids_ff.push_back(
                                0.5 * (pdf_11_0 + pdf_0_11 + BETA_0 * (pdf_1_0 + pdf_0_1) + 2. * pdf_1_1) / &pdf
                                    * lo_grid.grid.clone()
                                    * delta_lo_norm,
                            );
                        }
                        if grid_rf.is_none() {
                            tracing::debug!("Reconstructing RF grid");
                            if pdfc_0_1.is_none() || pdfc_1_0.is_none() {
                                pdfc_0_1 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 0, 1));
                                pdfc_1_0 = Some(self.build_flex_pdf_grid(block, &hp, &mu_f, 1, 0));
                            }
                            let pdf_0_1 = pdfc_0_1.as_ref().unwrap();
                            let pdf_1_0 = pdfc_1_0.as_ref().unwrap();
                            let lo_grid = self.lower_order_grid(pdf_info, *alphas_power, 2);
                            let delta_lo_norm = weight_info.as_ref().unwrap().norm as Float / lo_grid.norm;
                            grids_rf.push_back(
                                -(p - 1.) * BETA_0 * (pdf_0_1 + pdf_1_0) / &pdf * lo_grid.grid.clone() * delta_lo_norm,
                            );
                        }
                    }
                },
            }
        }
        // Assign prepared grids to self
        for block in self.blocks.iter_mut() {
            match &mut block.data {
                BlockData::DataBlock { .. } => todo!(),
                BlockData::MultBlock { .. } => todo!(),
                BlockData::TheoryBlock { grid, .. } => match grid {
                    Grid::Fixed { .. } => todo!(),
                    Grid::Flex {
                        grid_f,
                        grid_r,
                        grid_rr,
                        grid_ff,
                        grid_rf,
                        ..
                    } => {
                        if block.scale_format <= 4 {
                            // LO grid, nothing more to do
                            continue;
                        }
                        if grid_f.is_none() {
                            *grid_f = Some(grids_f.pop_front().unwrap());
                        }
                        if grid_r.is_none() {
                            *grid_r = Some(grids_r.pop_front().unwrap());
                        }
                        if block.scale_format <= 5 {
                            // NLO grid, nothing more to do
                            continue;
                        }
                        if grid_rr.is_none() {
                            *grid_rr = Some(grids_rr.pop_front().unwrap());
                        }
                        if block.scale_format <= 6 {
                            // NNLO grid with only RR, nothing more to do
                            continue;
                        }
                        if grid_ff.is_none() {
                            *grid_ff = Some(grids_ff.pop_front().unwrap());
                        }
                        if grid_rf.is_none() {
                            *grid_rf = Some(grids_rf.pop_front().unwrap());
                        }
                    }
                },
            }
        }
        for (i, block) in self.blocks.iter().enumerate() {
            match &block.data {
                BlockData::TheoryBlock { grid, .. } => match grid {
                    Grid::Flex {
                        grid_f,
                        grid_r,
                        grid_rr,
                        grid_ff,
                        grid_rf,
                        ..
                    } => tracing::debug!(
                        "Block {i}: {}, {}, {}, {}, {}",
                        grid_f.is_some(),
                        grid_r.is_some(),
                        grid_rr.is_some(),
                        grid_ff.is_some(),
                        grid_rf.is_some()
                    ),
                    _ => todo!(),
                },
                _ => todo!(),
            }
        }

        return Ok(());
    }
}
