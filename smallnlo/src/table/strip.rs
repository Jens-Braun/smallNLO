use super::{Block, BlockData, FastNLOFile, Grid};

impl FastNLOFile {
    pub fn strip(&mut self) {
        for b in self.blocks.iter_mut() {
            b.strip();
        }
    }
}

impl Block {
    pub fn strip(&mut self) {
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
