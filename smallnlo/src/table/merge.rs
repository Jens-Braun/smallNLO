use crate::table::{BlockData, FastNLOFile, Grid};

/// This implementation assumes the format of the merged tables to be identical for simplicity

impl FastNLOFile {
    pub fn merge(&mut self, other: &FastNLOFile) {
        for (b, b_other) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            match (&mut b.data, &b_other.data) {
                (BlockData::TheoryBlock { grid, .. }, BlockData::TheoryBlock { grid: grid_other, .. }) => {
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
                            *grid += grid_other;
                            if let Some(g_f) = grid_f
                                && let Some(g_f_other) = grid_f_other
                            {
                                *g_f += g_f_other;
                            }
                            if let Some(g_r) = grid_r
                                && let Some(g_r_other) = grid_r_other
                            {
                                *g_r += g_r_other;
                            }
                            if let Some(g_rr) = grid_rr
                                && let Some(g_rr_other) = grid_rr_other
                            {
                                *g_rr += g_rr_other;
                            }
                            if let Some(g_ff) = grid_ff
                                && let Some(g_ff_other) = grid_ff_other
                            {
                                *g_ff += g_ff_other;
                            }
                            if let Some(g_rf) = grid_rf
                                && let Some(g_rf_other) = grid_rf_other
                            {
                                *g_rf += g_rf_other;
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
}
