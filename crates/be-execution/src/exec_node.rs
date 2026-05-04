use common::Result;
use types::{Block, Bitmap, Vector, Schema};
use std::sync::{Arc, Mutex};
use be_storage::tablet::{Tablet, TabletSchema, truncate_tablet};

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
    pub build_keys: Vec<usize>,
    pub probe_keys: Vec<usize>,
    pub build_child: Box<dyn ExecNode>,
    pub probe_child: Box<dyn ExecNode>,
    pub build_schema: Schema,
    pub probe_schema: Schema,
    pub opened: bool,
    pub build_complete: bool,
    pub probe_consumed: bool,
    pub hash_table: std::collections::HashMap<String, Vec<Block>>,
    pub current_probe_blocks: Vec<Block>,
    pub current_probe_idx: usize,
    pub matched_build_keys: std::collections::HashSet<String>,
}

impl HashJoinExecNode {
    pub fn new(
        join_type: String,
        build_keys: Vec<usize>,
        probe_keys: Vec<usize>,
        build_child: Box<dyn ExecNode>,
        probe_child: Box<dyn ExecNode>,
        build_schema: Schema,
        probe_schema: Schema,
    ) -> Self {
        Self {
            join_type,
            build_keys,
            probe_keys,
            build_child,
            probe_child,
            build_schema,
            probe_schema,
            opened: false,
            build_complete: false,
            probe_consumed: false,
            hash_table: std::collections::HashMap::new(),
            current_probe_blocks: Vec::new(),
            current_probe_idx: 0,
            matched_build_keys: std::collections::HashSet::new(),
        }
    }

    fn extract_keys_from_block(block: &Block, key_indices: &[usize]) -> Vec<String> {
        (0..block.num_rows()).map(|row_idx| {
            let mut key_parts = Vec::new();
            for &idx in key_indices {
                if idx < block.num_columns() {
                    if let Some(col) = block.column(idx) {
                        let scalar = col.scalar_at(row_idx);
                        key_parts.push(format!("{:?}", scalar));
                    }
                }
            }
            key_parts.join("|")
        }).collect()
    }
}

impl ExecNode for HashJoinExecNode {
    fn open(&mut self) -> Result<()> {
        self.build_child.open()?;
        self.probe_child.open()?;
        self.opened = true;
        self.build_complete = false;
        self.probe_consumed = false;
        self.hash_table.clear();
        self.current_probe_blocks.clear();
        self.current_probe_idx = 0;
        self.matched_build_keys.clear();
        Ok(())
    }

    fn get_next(&mut self) -> Result<Option<Block>> {
        // Build phase: read all build blocks and populate hash table
        if !self.build_complete {
            let mut build_blocks = Vec::new();
            while let Some(block) = self.build_child.get_next()? {
                build_blocks.push(block);
            }

            // Build hash table: key -> blocks
            for block in &build_blocks {
                let keys = Self::extract_keys_from_block(block, &self.build_keys);
                for (row_idx, key) in keys.iter().enumerate() {
                    let row_block = block.slice(row_idx, 1);
                    self.hash_table.entry(key.clone()).or_insert_with(Vec::new).push(row_block);
                }
            }

            self.build_complete = true;
            tracing::debug!("HashJoin build complete: {} keys in hash table", self.hash_table.len());
        }

        // Probe phase: read probe blocks and join with hash table
        while let Some(block) = self.probe_child.get_next()? {
            let keys = Self::extract_keys_from_block(&block, &self.probe_keys);
            let mut result_blocks = Vec::new();

            for (row_idx, key) in keys.iter().enumerate() {
                if let Some(build_blocks) = self.hash_table.get(key) {
                    // Found match - concatenate probe row with each matching build row
                    self.matched_build_keys.insert(key.clone());
                    let probe_row = block.slice(row_idx, 1);
                    for build_row in build_blocks {
                        let mut joined = probe_row.clone();
                        joined.append_block(build_row);
                        result_blocks.push(joined);
                    }
                }
                // For INNER join: only output rows with matches
                // For LEFT OUTER join: would need to track unmatched rows - simplified for now
            }

            if !result_blocks.is_empty() {
                // Concatenate all result blocks
                return Ok(Some(Block::concat(&result_blocks).unwrap_or_else(|| block)));
            }
        }

        // For LEFT OUTER join - output unmatched build rows
        if self.join_type == "LEFT" && !self.matched_build_keys.is_empty() {
            // This would need to track unmatched build keys - simplified for now
        }

        Ok(None)
    }

    fn close(&mut self) -> Result<()> {
        self.build_child.close()?;
        self.probe_child.close()?;
        self.opened = false;
        self.build_complete = false;
        self.probe_consumed = false;
        self.hash_table.clear();
        self.current_probe_blocks.clear();
        self.matched_build_keys.clear();
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

        // In a real implementation, this would:
        // 1. Look up the tablet for the given database.table
        // 2. Call truncate_tablet(&tablet) to clear all data
        //
        // The tablet lookup would go through the catalog/table service:
        // let tablet = catalog.get_table(&self.database, &self.table_name)?;
        // let tablet = tablet.read()?;
        // truncate_tablet(&tablet)?;
        //
        // For now, we log the truncate operation
        if self.if_exists {
            tracing::info!("TRUNCATE TABLE IF EXISTS {}.{} executed", self.database, self.table_name);
        } else {
            tracing::info!("TRUNCATE TABLE {}.{}", self.database, self.table_name);
        }

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
