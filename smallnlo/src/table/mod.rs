use std::fmt::Write;
use std::path::Path;

use ndarray::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Float;
use crate::error::{ReadError, WriteError};
use crate::table::writer::write_fastnlo;

mod merge;
mod reader;
mod reconstruct;
mod strip;
mod writer;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FastNLOFile {
    pub metadata: Metadata,
    pub bin_info: BinInfo,
    pub blocks: Vec<Block>,
}

impl FastNLOFile {
    pub fn read(file: &Path) -> Result<Self, ReadError> {
        return reader::read_fastnlo(file);
    }

    pub fn read_str(content: &str) -> Result<Self, ReadError> {
        return reader::read_fastnlo_str(content);
    }

    pub fn write_file(&self, path: &Path) -> Result<(), WriteError> {
        let mut buf = String::new();
        writer::write_fastnlo(&mut buf, self)?;
        std::fs::write(path, &buf)?;
        return Ok(());
    }

    pub fn write(&self, w: &mut impl Write) -> Result<(), WriteError> {
        return write_fastnlo(w, self);
    }

    pub fn set_merge_weights(&mut self, weights: &[f64]) {
        assert!(weights.len() == self.bin_info.bins.len());
        for b in &mut self.blocks {
            b.set_merge_weights(weights);
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Metadata {
    pub table_version: usize,     // ltabversion
    pub scenario_name: String,    // ScenName
    pub n_contrib: usize,         // NContrib
    pub n_mult: usize,            // Nmult
    pub n_data: usize,            // Ndata
    pub n_user: usize,            // NUserBlocks
    pub unit: usize,              // lpublunits
    pub description: Vec<String>, // ScDescript
    pub cms_energy: f64,          // Ecms
    pub alphas_ord: usize,        // ILOord
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BinInfo {
    pub bins: Vec<Bin>,
    pub dim_labels: Vec<String>,
    pub diff_bin: Vec<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Bin {
    pub values: Vec<BinPosition>,
    pub size: f64, // BinSize
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum BinPosition {
    Central(f64),                       // LoBin
    Boundaries { low: f64, high: f64 }, // LoBin, HiBin
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Block {
    pub unit: usize,                   // IXsectUnits
    pub data_block: bool,              // IDataFlag
    pub mult_block: bool,              // IAddMultFlag
    pub contribution_type: usize,      // IContrFlag1
    pub contribution_order: usize,     // IContrFlag2
    pub scale_format: usize,           // NscaleDep
    pub description: Vec<String>,      // CtrbDescript
    pub code_description: Vec<String>, // CodeDescript
    pub data: BlockData,
    pub coeff_info_flags_1: Vec<usize>,            // ICoeffInfoBlockFlag1
    pub coeff_info_flags_2: Vec<usize>,            // ICoeffInfoBlockFlag2
    pub coeff_block_description: Vec<Vec<String>>, // CoeffInfoBlockDescript
    pub coeff_block_content: Vec<Vec<f64>>,        // CoeffInfoBlockContent
}

impl Block {
    pub fn set_merge_weights(&mut self, weights: &[f64]) {
        match &mut self.data {
            BlockData::TheoryBlock {
                weight_info, pdf_info, ..
            } => {
                if let Some(wi) = weight_info.as_mut() {
                    let w = Array2::from_shape_fn((pdf_info.n_subproc, weights.len()), |(_, i)| weights[i]);
                    wi.sig_obs = w;
                }
            }
            _ => (),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum BlockData {
    DataBlock {
        uncorr_sources: Vec<String>, // UncDescr
        corr_sources: Vec<String>,   // CorDescr
        centers: Array1<Float>,      // Xcenter
        values: Array1<Float>,       // value
        uncorr_low: Array2<Float>,   // UnCorLo
        uncorr_high: Array2<Float>,  // UnCorHi
        corr_low: Array2<Float>,     // CorrLo
        corr_high: Array2<Float>,    // CorrHi
        corr_matrix: Array2<Float>,  // matrixelement
    },
    MultBlock {
        uncorr_sources: Vec<String>, // UncDescr
        corr_sources: Vec<String>,   // CorDescr
        centers: Array1<Float>,      // Xcenter
        values: Array1<Float>,       // value
        uncorr_low: Array2<Float>,   // UnCorLo
        uncorr_high: Array2<Float>,  // UnCorHi
        corr_low: Array2<Float>,     // CorrLo
        corr_high: Array2<Float>,    // CorrHi
    },
    TheoryBlock {
        reference_table: bool,     // IRef
        i_scale_dependence: usize, // IScaleDep
        n_events: isize,           // Nevt
        weight_info: Option<WeightInfo>,
        alphas_power: usize, // Npow
        pdf_info: PDFInfo,
        //n_events_bins: Vec<Vec<usize>>,      // NEvtBinProc
        x1_nodes: Vec<Vec<Float>>,           // XNode1
        x2_nodes: Vec<Vec<Float>>,           // XNode2
        z_nodes: Vec<Vec<Float>>,            // Znode
        scale_dimension: Vec<usize>,         // Iscale
        scale_description: Vec<Vec<String>>, // ScaleDescript
        grid: Grid,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WeightInfo {
    pub n_events: f64,               // NEvt
    pub norm: f64,                   // WgtNevt
    pub n_tables: usize,             // NumTable
    pub n_entries: usize,            // WgtNumEv
    pub sum_weights_sq: f64,         // WgtSumW2
    pub sum_sig_sq: f64,             // SigSumW2
    pub sum_sig: f64,                // SigSum
    pub weight_sq_obs: Array2<f64>,  // WgtObsSumW2
    pub sig_sq_obs: Array2<f64>,     // SigObsSumW2
    pub sig_obs: Array2<f64>,        // SigObsSum
    pub n_events_obs: Array2<usize>, // WgtObsNumEv
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PDFInfo {
    pub pdfs: Vec<usize>,                         // NPDFDG
    pub n_pdf_dim: usize,                         // NPDFDim
    pub fragmentation_functions: Vec<usize>,      // NFFPDG
    pub n_ff_dim: usize,                          // NFFDim
    pub n_subproc: usize,                         // NSubproc
    pub pdf_flag_1: usize,                        // IPDFdef1
    pub pdf_flag_2: usize,                        // IPDFdef2
    pub pdf_flag_3: usize,                        // IPDFdef3
    pub parton_flavors: Vec<Vec<(isize, isize)>>, // PDF1Flavor, PDF2Flavor
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Grid {
    Fixed {
        n_scale_var: Vec<usize>,   // Nscalevar
        n_scale_node: Vec<usize>,  // Nscalenode
        scale_fac: Array2<Float>,  // ScaleFac
        scale_node: Array4<Float>, // ScaleNode
        grid: Array5<Float>,       // SigmaTilde
    },
    Flex {
        scale_node_1: Array2<Float>,    // ScaleNode1
        scale_node_2: Array2<Float>,    // ScaleNode2
        grid: Array5<Float>,            // SigmaTileMuIndep
        grid_f: Option<Array5<Float>>,  // SigmaTildeFDep
        grid_r: Option<Array5<Float>>,  // SigmaTildeRDep
        grid_rr: Option<Array5<Float>>, // SigmaTildeRRDep
        grid_ff: Option<Array5<Float>>, // SigmaTildeFFDep
        grid_rf: Option<Array5<Float>>, // SigmaTildeRFDep
        sigma_ref_mixed: Array2<Float>, // SigmaRefMixed (undocumented)
        sigma_ref_s1: Array2<Float>,    // SigmaRefMixed (undocumented)
        sigma_ref_s2: Array2<Float>,    // SigmaRefMixed (undocumented)
    },
}

impl Grid {
    pub(crate) fn grids<'s>(&'s self) -> impl Iterator<Item = ArrayView5<'s, Float>> {
        match self {
            Self::Fixed { grid, .. } => {
                return vec![Some(grid.view())].into_iter().flatten();
            }
            Self::Flex {
                grid,
                grid_f,
                grid_r,
                grid_rr,
                grid_ff,
                grid_rf,
                ..
            } => vec![
                Some(grid.view()),
                grid_f.as_ref().map(|g| g.view()),
                grid_r.as_ref().map(|g| g.view()),
                grid_rr.as_ref().map(|g| g.view()),
                grid_ff.as_ref().map(|g| g.view()),
                grid_rf.as_ref().map(|g| g.view()),
            ]
            .into_iter()
            .flatten(),
        }
    }

    pub(crate) fn grids_mut<'s>(&'s mut self) -> impl Iterator<Item = ArrayViewMut5<'s, Float>> {
        match self {
            Self::Fixed { grid, .. } => {
                return vec![Some(grid.view_mut())].into_iter().flatten();
            }
            Self::Flex {
                grid,
                grid_f,
                grid_r,
                grid_rr,
                grid_ff,
                grid_rf,
                ..
            } => vec![
                Some(grid.view_mut()),
                grid_f.as_mut().map(|g| g.view_mut()),
                grid_r.as_mut().map(|g| g.view_mut()),
                grid_rr.as_mut().map(|g| g.view_mut()),
                grid_ff.as_mut().map(|g| g.view_mut()),
                grid_rf.as_mut().map(|g| g.view_mut()),
            ]
            .into_iter()
            .flatten(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserBlock {
    pub user_flag: usize,         // IUserFlag
    pub description: Vec<String>, // UserBlockDescr
    pub lines: Vec<String>,       // UserLines
}

#[cfg(test)]
mod tests {
    use super::FastNLOFile;
    use std::path::PathBuf;

    #[test]
    fn reader_test() {
        let file = PathBuf::from(
            "/home/jens/KIT/N3LO_Grid/2jetfc.NNLO.fnl3832-fc-v2_yb0_ys0_ptavgj12_arxiv-1705.02628_v25.tab",
        );
        let tab = FastNLOFile::read(&file);
        match tab {
            Err(e) => {
                println!("{e}");
                panic!();
            }
            Ok(t) => {
                println!(
                    "{:?}",
                    match &t.blocks[2].data {
                        super::BlockData::TheoryBlock { grid, .. } => match grid {
                            super::Grid::Flex { grid, .. } => grid[[4, 120, 3, 0, 7]],
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    }
                )
            }
        }
    }
}
