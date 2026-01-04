use std::path::PathBuf;

use ndarray::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Float;
use crate::error::ReadError;

mod reader;

#[derive(Debug, Deserialize, Serialize)]
pub struct FastNLOFile {
    metadata: Metadata,
    bins: Vec<Bin>,
    blocks: Vec<Block>,
}

impl FastNLOFile {
    pub fn read(file: PathBuf) -> Result<Self, ReadError> {
        return reader::read_fastnlo(file);
    }

    pub fn strip(&mut self) {
        for b in self.blocks.iter_mut() {
            b.strip();
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    table_version: usize,     // ltabversion
    scenario_name: String,    // ScenName
    n_contrib: usize,         // NContrib
    n_mult: usize,            // Nmult
    n_data: usize,            // Ndata
    n_user: usize,            // NUserBlocks
    unit: usize,              // lpublunits
    description: Vec<String>, // ScDescript
    cms_energy: Float,        // Ecms
    alphas_ord: usize,        // ILOord
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Bin {
    values: Vec<BinInfo>,
    size: Float, // BinSize
}

#[derive(Debug, Deserialize, Serialize)]
pub enum BinInfo {
    Central(Float),                         // LoBin
    Boundaries { low: Float, high: Float }, // LoBin, HiBin
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Block {
    unit: usize,                   // IXsectUnits
    data_block: bool,              // IDataFlag
    mult_block: bool,              // IAddMultFlag
    contribution_type: usize,      // IContrFlag1
    contribution_order: usize,     // IContrFlag2
    scale_format: usize,           // NscaleDep
    description: Vec<String>,      // CtrbDescript
    code_description: Vec<String>, // CodeDescript
    data: BlockData,
    coeff_info_flags_1: Vec<usize>,            // ICoeffInfoBlockFlag1
    coeff_info_flags_2: Vec<usize>,            // ICoeffInfoBlockFlag2
    coeff_block_description: Vec<Vec<String>>, // CoeffInfoBlockDescript
    coeff_block_content: Vec<Vec<Float>>,      // CoeffInfoBlockContent
}

impl Block {
    pub(crate) fn strip(&mut self) {
        match self.data {
            BlockData::TheoryBlock { ref mut grid, .. } => match *grid {
                Grid::Flex {
                    ref mut grid_f,
                    ref mut grid_r,
                    ref mut grid_ff,
                    ref mut grid_rf,
                    ref mut grid_rr,
                    ..
                } => {
                    *grid_f = None;
                    *grid_r = None;
                    *grid_ff = None;
                    *grid_rr = None;
                    *grid_rf = None;
                }
                _ => (),
            },
            _ => (),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
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
        reference_table: bool,   // IRef
        scale_dependence: usize, // IScaleDep
        n_events: isize,         // Nevt
        weight_info: Option<WeightInfo>,
        alphas_power: usize, // Npow
        pdf_info: PDFInfo,
        //n_events_bins: Vec<Vec<usize>>,      // NEvtBinProc
        x1_nodes: Vec<Vec<Float>>,           // XNode1
        x2_nodes: Vec<Vec<Float>>,           // XNode 2
        z_nodes: Vec<Vec<Float>>,            // Znode
        scale_dimension: Vec<usize>,         // Iscale
        scale_description: Vec<Vec<String>>, // ScaleDescript
        grid: Grid,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WeightInfo {
    norm: Float,                    // WgtNevt
    n_tables: usize,                // NumTable
    n_entries: usize,               // WgtNumEv
    sum_weights_sq: Float,          // WgtSumW2
    sum_sig_sq: Float,              // SigSumW2
    sum_sig: Float,                 // SigSum
    weight_sq_obs: Vec<Vec<Float>>, // WgtObsSumW2
    sig_sq_obs: Vec<Vec<Float>>,    // SigObsSumW2
    sig_obs: Vec<Vec<Float>>,       // SigObsSum
    n_events_obs: Vec<Vec<usize>>,  // WgtObsNumEv
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PDFInfo {
    pdfs: Vec<usize>,                         // NPDFDG
    n_pdf_dim: usize,                         // NPDFDim
    fragmentation_functions: Vec<usize>,      // NFFPDG
    n_ff_dim: usize,                          // NFFDim
    n_subproc: usize,                         // NSubproc
    pdf_flag_1: usize,                        // IPDFdef1
    pdf_flag_2: usize,                        // IPDFdef2
    pdf_flag_3: usize,                        // IPDFdef3
    parton_flavors: Vec<Vec<(isize, isize)>>, // PDF1Flavor, PDF2Flavor
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Grid {
    Fixed {
        n_scale_var: Vec<usize>,   // Nscalevar
        n_scale_node: Vec<usize>,  // Nscalenode
        scale_fac: Array2<Float>,  // ScaleFac
        scale_node: Array4<Float>, // ScaleNode
        grid: Array6<Float>,       // SigmaTilde
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

#[derive(Debug, Deserialize, Serialize)]
pub struct UserBlock {
    user_flag: usize,         // IUserFlag
    description: Vec<String>, // UserBlockDescr
    lines: Vec<String>,       // UserLines
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
        let tab = FastNLOFile::read(file);
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
