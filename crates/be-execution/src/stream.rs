use common::Result;
use types::Block;

pub struct RowBatchStream {
    blocks: Vec<Block>,
    index: usize,
}

impl RowBatchStream {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks, index: 0 }
    }

    pub fn has_next(&self) -> bool {
        self.index < self.blocks.len()
    }

    pub fn next(&mut self) -> Option<&Block> {
        if self.index < self.blocks.len() {
            let block = &self.blocks[self.index];
            self.index += 1;
            Some(block)
        } else {
            None
        }
    }
}
