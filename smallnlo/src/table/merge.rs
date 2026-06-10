use crate::{
    Float,
    error::MergeError,
    table::{BlockData, FastNLOFile, Grid},
};
use ndarray::prelude::*;

/// This implementation assumes the format of the merged tables to be identical for simplicity

impl FastNLOFile {
    pub fn merge(&mut self, other: &FastNLOFile, weighted: bool) -> Result<(), MergeError> {
        if self.metadata.alphas_ord != other.metadata.alphas_ord {
            return Err(MergeError::IncompatibleTables(
                "alphas_ord".to_owned(),
                self.metadata.alphas_ord.to_string(),
                other.metadata.alphas_ord.to_string(),
            ));
        }
        if self.metadata.description != other.metadata.description {
            return Err(MergeError::IncompatibleTables(
                "description".to_owned(),
                format!("{:?}", self.metadata.description),
                format!("{:?}", other.metadata.description),
            ));
        }
        let mut seen = vec![false; other.blocks.len()];
        for b in self.blocks.iter_mut() {
            let (i, b_other) = other
                .blocks
                .iter()
                .enumerate()
                .find(|(_, o)| match (&b.data, &o.data) {
                    (
                        BlockData::TheoryBlock { alphas_power, .. },
                        BlockData::TheoryBlock {
                            alphas_power: other, ..
                        },
                    ) => alphas_power == other,
                    (_, _) => false,
                })
                .unzip();
            if let Some(b_other) = b_other {
                seen[i.unwrap()] = true;
                match (&mut b.data, &b_other.data) {
                    (
                        BlockData::TheoryBlock {
                            grid,
                            weight_info,
                            pdf_info,
                            ..
                        },
                        BlockData::TheoryBlock {
                            grid: grid_other,
                            weight_info: weight_info_other,
                            ..
                        },
                    ) => {
                        let norm = weight_info.as_ref().map(|w| w.norm).unwrap_or(1.).recip();
                        let norm_other = weight_info_other.as_ref().map(|w| w.norm).unwrap_or(1.).recip();
                        if weighted {
                            let w1 = if let Some(wi) = weight_info {
                                wi.sig_obs.clone()
                            } else {
                                Array2::from_elem((pdf_info.n_subproc, self.bin_info.bins.len()), 1.0f64)
                            };
                            let w2 = if let Some(wi) = weight_info_other {
                                wi.sig_obs.clone()
                            } else {
                                Array2::from_elem((pdf_info.n_subproc, self.bin_info.bins.len()), 1.0f64)
                            };
                            grid.grids_mut().zip(grid_other.grids()).for_each(|(g, go)| {
                                azip!((index (i, _, _, _, n), x in g, y in go) {
                                    *x = (((*x as f64) * norm * w1[[n, i]] + (*y as f64) * norm_other * w2[[n, i]]) / (w1[[n, i]] + w2[[n, i]]) * (norm.recip() + norm_other.recip())) as Float;
                                })
                            });
                        } else {
                            grid.grids_mut().zip(grid_other.grids()).for_each(|(mut g, go)| {
                                g.zip_mut_with(&go, |x, y| {
                                    *x = ((*x as f64) * norm + (*y as f64) * norm_other) as Float
                                });
                            });
                        }
                        match (grid, grid_other) {
                            (
                                Grid::Flex {
                                    sigma_ref_mixed,
                                    sigma_ref_s1,
                                    sigma_ref_s2,
                                    ..
                                },
                                Grid::Flex {
                                    sigma_ref_mixed: sigma_ref_mixed_other,
                                    sigma_ref_s1: sigma_ref_s1_other,
                                    sigma_ref_s2: sigma_ref_s2_other,
                                    ..
                                },
                            ) => {
                                *sigma_ref_mixed += sigma_ref_mixed_other;
                                *sigma_ref_s1 += sigma_ref_s1_other;
                                *sigma_ref_s2 += sigma_ref_s2_other;
                            }
                            _ => todo!("only flex-type theory contributions are implemented"),
                        }
                        weight_info.as_mut().zip(weight_info_other.as_ref()).map(|(w, wo)| {
                            w.norm = if weighted { w.norm + wo.norm } else { 1. };
                            w.n_events = if weighted { w.norm + wo.norm } else { 1. };
                            w.n_entries += wo.n_entries;
                            w.n_tables += wo.n_tables;
                            w.sum_sig += wo.sum_sig;
                            w.sum_sig_sq += wo.sum_sig_sq;
                            w.sum_weights_sq += wo.sum_weights_sq;
                            w.n_events_obs += &wo.n_events_obs;
                            w.sig_obs += &wo.sig_obs;
                            w.sig_sq_obs += &wo.sig_sq_obs;
                            w.weight_sq_obs += &wo.weight_sq_obs;
                        });
                    }
                    _ => todo!("only flex-type theory contributions are implemented"),
                }
            }
        }
        let n_add_contrib = seen.iter().filter(|s| !**s).count();
        self.metadata.n_contrib += n_add_contrib;
        for (i, s) in seen.into_iter().enumerate() {
            if !s {
                self.blocks.push(other.blocks[i].clone());
            }
        }
        Ok(())
    }
}
