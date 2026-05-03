use common::Result;
use types::Block;

pub trait ExecNode: Send + Sync {
    fn open(&mut self) -> Result<()>;
    fn get_next(&mut self) -> Result<Option<Block>>;
    fn close(&mut self) -> Result<()>;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct ScanExecNode {
    pub table_name: String,
    pub columns: Vec<String>,
    pub limit: Option<usize>,
    opened: bool,
}

impl ScanExecNode {
    pub fn new(table_name: String, columns: Vec<String>) -> Self {
        Self { table_name, columns, limit: None, opened: false }
    }
}

impl ExecNode for ScanExecNode {
    fn open(&mut self) -> Result<()> {
        self.opened = true;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // TODO: read from storage engine
        Ok(None)
    }

    fn close(&mut self) -> Result<()> {
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct FilterExecNode {
    pub predicate: String,
    pub child: Box<dyn ExecNode>,
}

impl ExecNode for FilterExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // TODO: apply filter
        self.child.get_next()
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ProjectExecNode {
    pub exprs: Vec<String>,
    pub child: Box<dyn ExecNode>,
}

impl ExecNode for ProjectExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // TODO: apply projection
        self.child.get_next()
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct AggregateExecNode {
    pub group_by: Vec<String>,
    pub aggregates: Vec<String>,
    pub child: Box<dyn ExecNode>,
}

impl ExecNode for AggregateExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // TODO: implement aggregation
        self.child.get_next()
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct SortExecNode {
    pub order_by: Vec<(String, bool)>,
    pub child: Box<dyn ExecNode>,
}

impl ExecNode for SortExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // TODO: implement sorting
        self.child.get_next()
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct LimitExecNode {
    pub limit: usize,
    pub offset: usize,
    pub child: Box<dyn ExecNode>,
    pub rows_returned: usize,
}

impl LimitExecNode {
    pub fn new(limit: usize, child: Box<dyn ExecNode>) -> Self {
        Self { limit, offset: 0, child, rows_returned: 0 }
    }
}

impl ExecNode for LimitExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        if self.rows_returned >= self.limit {
            return Ok(None);
        }
        self.child.get_next()
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
