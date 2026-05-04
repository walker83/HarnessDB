use crate::exec_node::{ExecNode, ExecutionPlan};
use common::Result;
use types::Block;

pub struct Pipeline {
    root: Box<ExecutionPlan>,
}

impl Pipeline {
    pub fn new(root: Box<ExecutionPlan>) -> Self {
        Self { root }
    }

    pub async fn open(&mut self) -> Result<()> {
        self.root.open().await
    }

    pub async fn get_next(&mut self) -> Result<Option<Block>> {
        self.root.get_next().await
    }

    pub async fn close(&mut self) -> Result<()> {
        self.root.close().await
    }

    pub async fn execute(mut self) -> Result<Vec<Block>> {
        self.open().await?;
        let mut results = Vec::new();
        while let Some(block) = self.get_next().await? {
            results.push(block);
        }
        self.close().await?;
        Ok(results)
    }
}