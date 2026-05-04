use common::Result;
use types::{Block, Bitmap, Vector, Schema};
use std::sync::{Arc, Mutex};

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
    pub predicates: Vec<String>,
    pub data: Option<Block>,
    pub opened: bool,
    pub rows_consumed: usize,
}

impl ScanExecNode {
    pub fn new(table_name: String, columns: Vec<String>) -> Self {
        Self {
            table_name,
            columns,
            limit: None,
            predicates: Vec::new(),
            data: None,
            opened: false,
            rows_consumed: 0,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_predicates(mut self, predicates: Vec<String>) -> Self {
        self.predicates = predicates;
        self
    }
}

impl ExecNode for ScanExecNode {
    fn open(&mut self) -> Result<()> {
        self.opened = true;
        self.rows_consumed = 0;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // Return the data if available
        if let Some(data) = self.data.take() {
            // Apply limit if set
            if let Some(limit) = self.limit {
                let rows_to_take = limit.saturating_sub(self.rows_consumed);
                if rows_to_take == 0 {
                    return Ok(None);
                }
                self.rows_consumed += data.num_rows();
                Ok(Some(data.slice(0, rows_to_take.min(data.num_rows()))))
            } else {
                Ok(Some(data))
            }
        } else {
            Ok(None)
        }
    }

    fn close(&mut self) -> Result<()> {
        self.opened = false;
        self.rows_consumed = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct FilterExecNode {
    pub predicate: String,
    pub child: Box<dyn ExecNode>,
    pub opened: bool,
}

impl ExecNode for FilterExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()?;
        self.opened = true;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        while let Some(block) = self.child.get_next()? {
            // Create a selection bitmap based on the predicate
            // For now, we'll implement a simple true/false filter
            let mut selection = Bitmap::with_capacity(block.num_rows());

            // TODO: Actually evaluate the predicate expression
            // For now, pass all rows through
            for _ in 0..block.num_rows() {
                selection.push(true);
            }

            let filtered = block.filter(&selection);
            if !filtered.is_empty() {
                return Ok(Some(filtered));
            }
        }
        Ok(None)
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ProjectExecNode {
    pub exprs: Vec<String>,
    pub child: Box<dyn ExecNode>,
    pub opened: bool,
}

impl ExecNode for ProjectExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()?;
        self.opened = true;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        self.child.get_next()
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct AggregateExecNode {
    pub group_by: Vec<String>,
    pub aggregates: Vec<String>,
    pub child: Box<dyn ExecNode>,
    pub opened: bool,
    pub returned: bool,
}

impl ExecNode for AggregateExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()?;
        self.opened = true;
        self.returned = false;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        // Consume all input blocks
        let mut all_blocks = Vec::new();
        while let Some(block) = self.child.get_next()? {
            all_blocks.push(block);
        }

        if all_blocks.is_empty() {
            return Ok(None);
        }

        self.returned = true;
        // TODO: Actually implement aggregation
        // For now, return the first block
        Ok(Some(all_blocks.into_iter().next().unwrap()))
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct SortExecNode {
    pub order_by: Vec<(String, bool)>,
    pub child: Box<dyn ExecNode>,
    pub opened: bool,
    pub buffered: Vec<Block>,
    pub returned: bool,
}

impl ExecNode for SortExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()?;
        self.opened = true;
        self.returned = false;
        self.buffered.clear();
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        // Buffer all input blocks
        while let Some(block) = self.child.get_next()? {
            self.buffered.push(block);
        }

        // TODO: Actually sort the blocks
        // For now, return concatenated blocks
        if self.buffered.is_empty() {
            return Ok(None);
        }

        self.returned = true;
        Ok(Some(Block::concat(&self.buffered).unwrap_or_else(|| {
            self.buffered.first().cloned().unwrap()
        })))
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()?;
        self.opened = false;
        self.buffered.clear();
        Ok(())
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
    pub rows_skipped: usize,
}

impl LimitExecNode {
    pub fn new(limit: usize, child: Box<dyn ExecNode>) -> Self {
        Self { limit, offset: 0, child, rows_returned: 0, rows_skipped: 0 }
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

impl ExecNode for LimitExecNode {
    fn open(&mut self) -> Result<()> {
        self.child.open()?;
        self.rows_returned = 0;
        self.rows_skipped = 0;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // Skip offset rows first
        while self.rows_skipped < self.offset {
            if let Some(block) = self.child.get_next()? {
                let remaining = self.offset - self.rows_skipped;
                if block.num_rows() <= remaining {
                    self.rows_skipped += block.num_rows();
                } else {
                    // Return the part after skipping
                    self.rows_skipped = self.offset;
                    let to_return = block.slice(remaining, block.num_rows() - remaining);
                    // Check against limit
                    let can_return = self.limit.saturating_sub(self.rows_returned);
                    if can_return == 0 {
                        return Ok(None);
                    }
                    if to_return.num_rows() <= can_return {
                        self.rows_returned += to_return.num_rows();
                        return Ok(Some(to_return));
                    } else {
                        self.rows_returned = self.limit;
                        return Ok(Some(to_return.slice(0, can_return)));
                    }
                }
            } else {
                return Ok(None);
            }
        }

        // Return rows up to limit
        if self.rows_returned >= self.limit {
            return Ok(None);
        }

        if let Some(block) = self.child.get_next()? {
            let can_return = self.limit - self.rows_returned;
            if block.num_rows() <= can_return {
                self.rows_returned += block.num_rows();
                Ok(Some(block))
            } else {
                self.rows_returned = self.limit;
                Ok(Some(block.slice(0, can_return)))
            }
        } else {
            Ok(None)
        }
    }

    fn close(&mut self) -> Result<()> {
        self.child.close()?;
        self.rows_returned = 0;
        self.rows_skipped = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// New node types

pub struct HashJoinExecNode {
    pub join_type: String,
    pub build_keys: Vec<String>,
    pub probe_keys: Vec<String>,
    pub build_child: Box<dyn ExecNode>,
    pub probe_child: Box<dyn ExecNode>,
    pub opened: bool,
    pub build_complete: bool,
    pub hash_table: std::collections::HashMap<String, Vec<Block>>,
}

impl HashJoinExecNode {
    pub fn new(
        join_type: String,
        build_keys: Vec<String>,
        probe_keys: Vec<String>,
        build_child: Box<dyn ExecNode>,
        probe_child: Box<dyn ExecNode>,
    ) -> Self {
        Self {
            join_type,
            build_keys,
            probe_keys,
            build_child,
            probe_child,
            opened: false,
            build_complete: false,
            hash_table: std::collections::HashMap::new(),
        }
    }
}

impl ExecNode for HashJoinExecNode {
    fn open(&mut self) -> Result<()> {
        self.build_child.open()?;
        self.probe_child.open()?;
        self.opened = true;
        self.build_complete = false;
        self.hash_table.clear();
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // Build phase
        if !self.build_complete {
            while let Some(block) = self.build_child.get_next()? {
                // TODO: Extract join keys and build hash table
                let key = "default".to_string();
                self.hash_table.entry(key).or_insert_with(Vec::new).push(block);
            }
            self.build_complete = true;
        }

        // Probe phase
        while let Some(block) = self.probe_child.get_next()? {
            // TODO: Probe hash table and join
            return Ok(Some(block));
        }

        Ok(None)
    }

    fn close(&mut self) -> Result<()> {
        self.build_child.close()?;
        self.probe_child.close()?;
        self.opened = false;
        self.build_complete = false;
        self.hash_table.clear();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct UnionExecNode {
    pub children: Vec<Box<dyn ExecNode>>,
    pub current_child: usize,
    pub opened: Vec<bool>,
}

impl UnionExecNode {
    pub fn new(children: Vec<Box<dyn ExecNode>>) -> Self {
        let opened = vec![false; children.len()];
        Self {
            children,
            current_child: 0,
            opened,
        }
    }
}

impl ExecNode for UnionExecNode {
    fn open(&mut self) -> Result<()> {
        for (i, child) in self.children.iter_mut().enumerate() {
            child.open()?;
            self.opened[i] = true;
        }
        self.current_child = 0;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        while self.current_child < self.children.len() {
            if let Some(block) = self.children[self.current_child].get_next()? {
                return Ok(Some(block));
            }
            self.current_child += 1;
        }
        Ok(None)
    }

    fn close(&mut self) -> Result<()> {
        for (child, opened) in self.children.iter_mut().zip(self.opened.iter()) {
            if *opened {
                child.close()?;
            }
        }
        self.opened = vec![false; self.children.len()];
        self.current_child = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// DDL execution nodes

pub struct TruncateExecNode {
    pub database: String,
    pub table_name: String,
    pub if_exists: bool,
    pub executed: bool,
}

impl TruncateExecNode {
    pub fn new(
        database: String,
        table_name: String,
        if_exists: bool,
    ) -> Self {
        Self {
            database,
            table_name,
            if_exists,
            executed: false,
        }
    }
}

impl ExecNode for TruncateExecNode {
    fn open(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        if self.executed {
            return Ok(None);
        }

        self.executed = true;

        // TODO: Actually truncate the table data
        // For now, this is a stub that returns success
        tracing::info!("Truncated table: {}.{}", self.database, self.table_name);

        // Return an empty result block
        Ok(None)
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
