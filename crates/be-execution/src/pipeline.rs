use crate::exec_node::ExecNode;
use common::Result;
use types::Block;

pub struct Pipeline {
    root: Box<dyn ExecNode>,
}

impl Pipeline {
    pub fn new(root: Box<dyn ExecNode>) -> Self {
        Self { root }
    }

    pub fn open(&mut self) -> Result<()> {
        self.root.open()
    }

    pub fn get_next(&mut self) -> Result<Option<Block>> {
        self.root.get_next()
    }

    pub fn close(&mut self) -> Result<()> {
        self.root.close()
    }

    pub fn execute(mut self) -> Result<Vec<Block>> {
        self.open()?;
        let mut results = Vec::new();
        while let Some(block) = self.get_next()? {
            results.push(block);
        }
        self.close()?;
        Ok(results)
    }
}
