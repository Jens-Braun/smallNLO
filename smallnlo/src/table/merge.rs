use crate::{
    Float,
    error::MergeError,
    table::{BlockData, FastNLOFile, Grid},
};

/// This implementation assumes the format of the merged tables to be identical for simplicity

impl FastNLOFile {
    pub fn merge(&mut self, other: &FastNLOFile) -> Result<(), MergeError> {
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
                        BlockData::TheoryBlock { grid, weight_info, .. },
                        BlockData::TheoryBlock {
                            grid: grid_other,
                            weight_info: weight_info_other,
                            ..
                        },
                    ) => {
                        let norm = weight_info.as_ref().map(|w| w.norm).unwrap_or(1.).recip();
                        weight_info.as_mut().map(|w| {
                            w.norm = 1.;
                            w.n_events = 1.
                        });
                        let norm_other = weight_info_other.as_ref().map(|w| w.norm).unwrap_or(1.).recip();
                        match (grid, grid_other) {
                            (
                                Grid::Flex {
                                    grid,
                                    grid_f,
                                    grid_r,
                                    grid_rr,
                                    grid_ff,
                                    grid_rf,
                                    sigma_ref_mixed,
                                    sigma_ref_s1,
                                    sigma_ref_s2,
                                    ..
                                },
                                Grid::Flex {
                                    grid: grid_other,
                                    grid_f: grid_f_other,
                                    grid_r: grid_r_other,
                                    grid_rr: grid_rr_other,
                                    grid_ff: grid_ff_other,
                                    grid_rf: grid_rf_other,
                                    sigma_ref_mixed: sigma_ref_mixed_other,
                                    sigma_ref_s1: sigma_ref_s1_other,
                                    sigma_ref_s2: sigma_ref_s2_other,
                                    ..
                                },
                            ) => {
                                *grid = grid.mapv(|x| ((x as f64) * norm) as Float)
                                    + grid_other.mapv(|x| ((x as f64) * norm_other) as Float);
                                if let Some(g_f) = grid_f
                                    && let Some(g_f_other) = grid_f_other
                                {
                                    *g_f = g_f.mapv(|x| ((x as f64) * norm) as Float)
                                        + g_f_other.mapv(|x| ((x as f64) * norm_other) as Float);
                                }
                                if let Some(g_r) = grid_r
                                    && let Some(g_r_other) = grid_r_other
                                {
                                    *g_r = g_r.mapv(|x| ((x as f64) * norm) as Float)
                                        + g_r_other.mapv(|x| ((x as f64) * norm_other) as Float);
                                }
                                if let Some(g_rr) = grid_rr
                                    && let Some(g_rr_other) = grid_rr_other
                                {
                                    *g_rr = g_rr.mapv(|x| ((x as f64) * norm) as Float)
                                        + g_rr_other.mapv(|x| ((x as f64) * norm_other) as Float);
                                }
                                if let Some(g_ff) = grid_ff
                                    && let Some(g_ff_other) = grid_ff_other
                                {
                                    *g_ff = g_ff.mapv(|x| ((x as f64) * norm) as Float)
                                        + g_ff_other.mapv(|x| ((x as f64) * norm_other) as Float);
                                }
                                if let Some(g_rf) = grid_rf
                                    && let Some(g_rf_other) = grid_rf_other
                                {
                                    *g_rf = g_rf.mapv(|x| ((x as f64) * norm) as Float)
                                        + g_rf_other.mapv(|x| ((x as f64) * norm_other) as Float);
                                }
                                *sigma_ref_mixed += sigma_ref_mixed_other;
                                *sigma_ref_s1 += sigma_ref_s1_other;
                                *sigma_ref_s2 += sigma_ref_s2_other;
                            }
                            _ => todo!("only flex-type theory contributions are implemented"),
                        }
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
