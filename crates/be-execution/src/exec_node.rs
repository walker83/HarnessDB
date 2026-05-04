use async_trait::async_trait;
use common::Result;
use types::{Block, Bitmap, Vector, Schema, ScalarValue};
use std::sync::{Arc, Mutex};
use be_storage::tablet::{Tablet, TabletSchema, truncate_tablet};
use be_storage::StorageEngine;
use be_storage::index::{ColumnPredicate, PredicateOp};

#[async_trait]
pub trait ExecNode: Send + Sync {
    async fn open(&mut self) -> Result<()>;
    async fn get_next(&mut self) -> Result<Option<Block>>;
    async fn close(&mut self) -> Result<()>;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub enum ExecutionPlan {
    Scan(ScanExecNode),
    Filter(FilterExecNode),
    Project(ProjectExecNode),
    Aggregate(AggregateExecNode),
    Sort(SortExecNode),
    Limit(LimitExecNode),
    HashJoin(HashJoinExecNode),
    Union(UnionExecNode),
    Truncate(TruncateExecNode),
    Window(WindowExecNode),
}

#[async_trait]
impl ExecNode for ExecutionPlan {
    async fn open(&mut self) -> Result<()> {
        match self {
            ExecutionPlan::Scan(node) => node.open().await,
            ExecutionPlan::Filter(node) => node.open().await,
            ExecutionPlan::Project(node) => node.open().await,
            ExecutionPlan::Aggregate(node) => node.open().await,
            ExecutionPlan::Sort(node) => node.open().await,
            ExecutionPlan::Limit(node) => node.open().await,
            ExecutionPlan::HashJoin(node) => node.open().await,
            ExecutionPlan::Union(node) => node.open().await,
            ExecutionPlan::Truncate(node) => node.open().await,
            ExecutionPlan::Window(node) => node.open().await,
        }
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        match self {
            ExecutionPlan::Scan(node) => node.get_next().await,
            ExecutionPlan::Filter(node) => node.get_next().await,
            ExecutionPlan::Project(node) => node.get_next().await,
            ExecutionPlan::Aggregate(node) => node.get_next().await,
            ExecutionPlan::Sort(node) => node.get_next().await,
            ExecutionPlan::Limit(node) => node.get_next().await,
            ExecutionPlan::HashJoin(node) => node.get_next().await,
            ExecutionPlan::Union(node) => node.get_next().await,
            ExecutionPlan::Truncate(node) => node.get_next().await,
            ExecutionPlan::Window(node) => node.get_next().await,
        }
    }

    async fn close(&mut self) -> Result<()> {
        match self {
            ExecutionPlan::Scan(node) => node.close().await,
            ExecutionPlan::Filter(node) => node.close().await,
            ExecutionPlan::Project(node) => node.close().await,
            ExecutionPlan::Aggregate(node) => node.close().await,
            ExecutionPlan::Sort(node) => node.close().await,
            ExecutionPlan::Limit(node) => node.close().await,
            ExecutionPlan::HashJoin(node) => node.close().await,
            ExecutionPlan::Union(node) => node.close().await,
            ExecutionPlan::Truncate(node) => node.close().await,
            ExecutionPlan::Window(node) => node.close().await,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        match self {
            ExecutionPlan::Scan(node) => node.as_any(),
            ExecutionPlan::Filter(node) => node.as_any(),
            ExecutionPlan::Project(node) => node.as_any(),
            ExecutionPlan::Aggregate(node) => node.as_any(),
            ExecutionPlan::Sort(node) => node.as_any(),
            ExecutionPlan::Limit(node) => node.as_any(),
            ExecutionPlan::HashJoin(node) => node.as_any(),
            ExecutionPlan::Union(node) => node.as_any(),
            ExecutionPlan::Truncate(node) => node.as_any(),
            ExecutionPlan::Window(node) => node.as_any(),
        }
    }
}

pub struct ScanExecNode {
    pub table_name: String,
    pub columns: Vec<String>,
    pub limit: Option<usize>,
    pub predicates: Vec<String>,
    pub data: Option<Block>,
    /// Optional tablet ID for storage-backed scan
    pub tablet_id: Option<u64>,
    /// Optional storage engine for reading from persistent storage
    pub storage: Option<Arc<StorageEngine>>,
    opened: bool,
    rows_consumed: usize,
}

impl ScanExecNode {
    pub fn new(table_name: String, columns: Vec<String>) -> Self {
        Self {
            table_name,
            columns,
            limit: None,
            predicates: Vec::new(),
            data: None,
            tablet_id: None,
            storage: None,
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

    /// Configure this scan to read from a storage engine using the given tablet_id.
    pub fn with_storage(mut self, tablet_id: u64, storage: Arc<StorageEngine>) -> Self {
        self.tablet_id = Some(tablet_id);
        self.storage = Some(storage);
        self
    }

    /// Build column projection indices from column names.
    fn build_projection(&self, schema: &Schema) -> Vec<usize> {
        if self.columns.is_empty() {
            (0..schema.num_fields()).collect()
        } else {
            self.columns.iter()
                .filter_map(|name| schema.index_of(name))
                .collect()
        }
    }

    /// Build predicates for storage read.
    fn build_predicates(&self) -> Vec<ColumnPredicate> {
        self.predicates.iter().filter_map(|p| Self::parse_predicate(p)).collect()
    }
    
    /// Parse a simple predicate string like "col = value".
    fn parse_predicate(pred_str: &str) -> Option<ColumnPredicate> {
        // Simple predicate parsing: "column op value"
        // Supported ops: =, <, <=, >, >=
        let parts: Vec<&str> = pred_str.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }
        
        let column_name = parts[0];
        let op_str = parts[1];
        let value_str = parts[2];
        
        let op = match op_str {
            "=" | "==" => PredicateOp::Eq,
            "<" => PredicateOp::Lt,
            "<=" => PredicateOp::Le,
            ">" => PredicateOp::Gt,
            ">=" => PredicateOp::Ge,
            _ => return None,
        };
        
        // Parse value (simple type inference)
        let value = if let Ok(n) = value_str.parse::<i64>() {
            ScalarValue::Int64(n)
        } else if let Ok(n) = value_str.parse::<i32>() {
            ScalarValue::Int32(n)
        } else if let Ok(f) = value_str.parse::<f64>() {
            ScalarValue::Float64(f)
        } else if value_str == "true" || value_str == "false" {
            ScalarValue::Boolean(value_str == "true")
        } else if value_str.starts_with("'") && value_str.ends_with("'") {
            // String literal: 'value'
            ScalarValue::String(value_str[1..value_str.len()-1].to_string())
        } else {
            // Treat as string
            ScalarValue::String(value_str.to_string())
        };
        
        Some(ColumnPredicate {
            column_name: column_name.to_string(),
            op,
            value,
        })
    }

    /// Read data from storage engine if configured.
    fn read_from_storage(&self) -> Result<Block> {
        let Some(tablet_id) = self.tablet_id else {
            return Ok(Block::empty(Schema::new(vec![])));
        };
        let Some(storage) = &self.storage else {
            return Ok(Block::empty(Schema::new(vec![])));
        };

        let projection = None; // Let storage return all columns, filter later
        let predicates = self.build_predicates();

        match storage.read_tablet(tablet_id, projection, &predicates) {
            Ok(block) => Ok(block),
            Err(e) => {
                tracing::warn!("Failed to read tablet {}: {}", tablet_id, e);
                Ok(Block::empty(Schema::new(vec![])))
            }
        }
    }
}

#[async_trait]
impl ExecNode for ScanExecNode {
    async fn open(&mut self) -> Result<()> {
        // If no pre-loaded data but we have storage configured, read from storage now
        if self.data.is_none() && self.storage.is_some() && self.tablet_id.is_some() {
            tracing::debug!("ScanExecNode: reading from storage tablet_id={}", self.tablet_id.unwrap());
            self.data = Some(self.read_from_storage()?);
        }
        self.opened = true;
        self.rows_consumed = 0;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if let Some(data) = self.data.take() {
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

    async fn close(&mut self) -> Result<()> {
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
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
}

#[async_trait]
impl ExecNode for FilterExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        while let Some(block) = self.child.get_next().await? {
            let mut selection = Bitmap::with_capacity(block.num_rows());

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

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ProjectExecNode {
    pub exprs: Vec<String>,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
}

#[async_trait]
impl ExecNode for ProjectExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        self.child.get_next().await
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct AggregateExecNode {
    pub group_by: Vec<usize>,
    pub aggregates: Vec<(String, usize)>,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
    pub returned: bool,
}

impl AggregateExecNode {
    fn compute_aggregate_batch(col: &Vector, func: &str) -> ScalarValue {
        match func {
            "count" => ScalarValue::Int64(col.count_batch() as i64),
            "sum" => col.sum_batch().unwrap_or(ScalarValue::Null),
            "min" => col.min_batch().unwrap_or(ScalarValue::Null),
            "max" => col.max_batch().unwrap_or(ScalarValue::Null),
            "avg" => col.avg_batch().unwrap_or(ScalarValue::Null),
            _ => ScalarValue::Null,
        }
    }

    fn compute_aggregate(values: &[ScalarValue], func: &str) -> ScalarValue {
        match func {
            "count" => ScalarValue::Int64(values.len() as i64),
            "sum" => {
                let mut sum: i64 = 0;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        sum += i;
                    }
                }
                ScalarValue::Int64(sum)
            }
            "min" => {
                let mut min: Option<i64> = None;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        min = Some(min.map(|m| m.min(*i)).unwrap_or(*i));
                    }
                }
                min.map(ScalarValue::Int64).unwrap_or(ScalarValue::Null)
            }
            "max" => {
                let mut max: Option<i64> = None;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        max = Some(max.map(|m| m.max(*i)).unwrap_or(*i));
                    }
                }
                max.map(ScalarValue::Int64).unwrap_or(ScalarValue::Null)
            }
            "avg" => {
                let mut sum: i64 = 0;
                let mut count: i64 = 0;
                for v in values {
                    if let ScalarValue::Int64(i) = v {
                        sum += i;
                        count += 1;
                    }
                }
                if count > 0 {
                    ScalarValue::Float64(sum as f64 / count as f64)
                } else {
                    ScalarValue::Null
                }
            }
            _ => ScalarValue::Null,
        }
    }
}

#[async_trait]
impl ExecNode for AggregateExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        self.returned = false;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        let mut all_blocks = Vec::new();
        while let Some(block) = self.child.get_next().await? {
            all_blocks.push(block);
        }

        if all_blocks.is_empty() {
            return Ok(None);
        }

        let combined = Block::concat(&all_blocks);
        if combined.is_none() {
            self.returned = true;
            return Ok(None);
        }
        let block = combined.unwrap();

        if self.group_by.is_empty() && self.aggregates.is_empty() {
            self.returned = true;
            return Ok(Some(block));
        }

        if self.group_by.is_empty() {
            let mut result_columns: Vec<Vector> = Vec::new();
            let mut result_schema_fields: Vec<types::Field> = Vec::new();

            for (func, col_idx) in &self.aggregates {
                if *col_idx < block.num_columns() {
                    if let Some(col) = block.column(*col_idx) {
                        let agg_value = Self::compute_aggregate_batch(col, func);
                        let vector = match agg_value {
                            ScalarValue::Int64(v) => Vector::Int64(types::vector::Int64Vector::from_vec(vec![v])),
                            ScalarValue::Float64(v) => Vector::Float64(types::vector::Float64Vector::from_vec(vec![v])),
                            ScalarValue::Int32(v) => Vector::Int32(types::vector::Int32Vector::from_vec(vec![v])),
                            _ => Vector::Null(types::vector::NullVector::new(1)),
                        };
                        result_columns.push(vector);
                        result_schema_fields.push(types::Field::new("", types::DataType::Null, true));
                    }
                }
            }

            self.returned = true;
            return Ok(Some(Block::new(Schema::new(result_schema_fields), result_columns)));
        }

        let mut groups: std::collections::HashMap<String, Vec<Vec<ScalarValue>>> = std::collections::HashMap::new();

        for row_idx in 0..block.num_rows() {
            let mut key_parts = Vec::new();
            for &col_idx in &self.group_by {
                if col_idx < block.num_columns() {
                    if let Some(col) = block.column(col_idx) {
                        key_parts.push(format!("{:?}", col.scalar_at(row_idx)));
                    }
                }
            }
            let group_key = key_parts.join("|");

            let row_values: Vec<ScalarValue> = (0..block.num_columns())
                .map(|col_idx| {
                    if let Some(col) = block.column(col_idx) {
                        col.scalar_at(row_idx)
                    } else {
                        ScalarValue::Null
                    }
                })
                .collect();

            groups.entry(group_key).or_insert_with(Vec::new).push(row_values);
        }

        let mut result_rows: Vec<Vec<ScalarValue>> = Vec::new();
        for (_group_key, group_rows) in &groups {
            let mut result_row = Vec::new();

            if !group_rows.is_empty() {
                for &col_idx in &self.group_by {
                    result_row.push(group_rows[0].get(col_idx).cloned().unwrap_or(ScalarValue::Null));
                }
            }

            for (func, col_idx) in &self.aggregates {
                let values: Vec<ScalarValue> = group_rows.iter()
                    .filter_map(|row| row.get(*col_idx).cloned())
                    .collect();
                result_row.push(Self::compute_aggregate(&values, func));
            }

            result_rows.push(result_row);
        }

        if result_rows.is_empty() {
            self.returned = true;
            return Ok(None);
        }

        let num_result_cols = self.group_by.len() + self.aggregates.len();
        let num_result_rows = result_rows.len();

        let mut columns: Vec<Vector> = Vec::new();
        for col_idx in 0..num_result_cols {
            let scalars: Vec<ScalarValue> = result_rows.iter()
                .filter_map(|row| row.get(col_idx).cloned())
                .collect();

            let vector = match scalars.first().unwrap_or(&ScalarValue::Null) {
                ScalarValue::Int64(_) => {
                    let data: Vec<i64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int64(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int64(types::vector::Int64Vector::from_vec(data))
                }
                ScalarValue::Float64(_) => {
                    let data: Vec<f64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Float64(f) = s { Some(*f) } else { None }
                    }).collect();
                    Vector::Float64(types::vector::Float64Vector::from_vec(data))
                }
                ScalarValue::Int32(_) => {
                    let data: Vec<i32> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int32(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int32(types::vector::Int32Vector::from_vec(data))
                }
                ScalarValue::String(_) => {
                    let data: Vec<String> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::String(s) = s { Some(s.clone()) } else { None }
                    }).collect();
                    let data_refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
                    Vector::String(types::vector::StringVector::from_vec(data_refs))
                }
                _ => Vector::Null(types::vector::NullVector::new(num_result_rows)),
            };
            columns.push(vector);
        }

        let schema_fields: Vec<types::Field> = (0..num_result_cols)
            .map(|_| types::Field::new("", types::DataType::Null, true))
            .collect();

        self.returned = true;
        Ok(Some(Block::new(Schema::new(schema_fields), columns)))
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct SortExecNode {
    pub order_by: Vec<(usize, bool)>,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
    pub buffered: Vec<Block>,
    pub returned: bool,
}

#[async_trait]
impl ExecNode for SortExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        self.returned = false;
        self.buffered.clear();
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        while let Some(block) = self.child.get_next().await? {
            self.buffered.push(block);
        }

        if self.buffered.is_empty() {
            return Ok(None);
        }

        let combined = Block::concat(&self.buffered);
        if combined.is_none() {
            return Ok(None);
        }
        let mut block = combined.unwrap();

        if self.order_by.is_empty() {
            self.returned = true;
            return Ok(Some(block));
        }

        let num_rows = block.num_rows();
        let mut indices: Vec<usize> = (0..num_rows).collect();

        let order_by = self.order_by.clone();
        let cmp_block = &block;
        indices.sort_unstable_by(|&a, &b| {
            for &(col_idx, ascending) in &order_by {
                if col_idx >= cmp_block.num_columns() {
                    continue;
                }
                if let Some(col) = cmp_block.column(col_idx) {
                    let ord = col.compare_at(a, b);
                    let ord = if ascending { ord } else { ord.reverse() };
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                }
            }
            std::cmp::Ordering::Equal
        });

        let mut sorted_block = Block::empty(block.schema().clone());
        for &idx in &indices {
            let row_block = block.slice(idx, 1);
            if sorted_block.is_empty() {
                sorted_block = row_block;
            } else {
                sorted_block.append_block(&row_block);
            }
        }

        self.returned = true;
        Ok(Some(sorted_block))
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
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
    pub child: Box<ExecutionPlan>,
    pub rows_returned: usize,
    pub rows_skipped: usize,
}

impl LimitExecNode {
    pub fn new(limit: usize, child: Box<ExecutionPlan>) -> Self {
        Self { limit, offset: 0, child, rows_returned: 0, rows_skipped: 0 }
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

#[async_trait]
impl ExecNode for LimitExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.rows_returned = 0;
        self.rows_skipped = 0;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        while self.rows_skipped < self.offset {
            if let Some(block) = self.child.get_next().await? {
                let remaining = self.offset - self.rows_skipped;
                if block.num_rows() <= remaining {
                    self.rows_skipped += block.num_rows();
                } else {
                    self.rows_skipped = self.offset;
                    let to_return = block.slice(remaining, block.num_rows() - remaining);
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

        if self.rows_returned >= self.limit {
            return Ok(None);
        }

        if let Some(block) = self.child.get_next().await? {
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

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.rows_returned = 0;
        self.rows_skipped = 0;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct HashJoinExecNode {
    pub join_type: String,
    pub build_keys: Vec<usize>,
    pub probe_keys: Vec<usize>,
    pub build_child: Box<ExecutionPlan>,
    pub probe_child: Box<ExecutionPlan>,
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
        build_child: Box<ExecutionPlan>,
        probe_child: Box<ExecutionPlan>,
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

#[async_trait]
impl ExecNode for HashJoinExecNode {
    async fn open(&mut self) -> Result<()> {
        self.build_child.open().await?;
        self.probe_child.open().await?;
        self.opened = true;
        self.build_complete = false;
        self.probe_consumed = false;
        self.hash_table.clear();
        self.current_probe_blocks.clear();
        self.current_probe_idx = 0;
        self.matched_build_keys.clear();
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if !self.build_complete {
            let mut build_blocks = Vec::new();
            while let Some(block) = self.build_child.get_next().await? {
                build_blocks.push(block);
            }

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

        while let Some(block) = self.probe_child.get_next().await? {
            let keys = Self::extract_keys_from_block(&block, &self.probe_keys);
            let mut result_blocks = Vec::new();

            for (row_idx, key) in keys.iter().enumerate() {
                if let Some(build_blocks) = self.hash_table.get(key) {
                    self.matched_build_keys.insert(key.clone());
                    let probe_row = block.slice(row_idx, 1);
                    for build_row in build_blocks {
                        let mut joined = probe_row.clone();
                        joined.append_block(build_row);
                        result_blocks.push(joined);
                    }
                }
            }

            if !result_blocks.is_empty() {
                return Ok(Some(Block::concat(&result_blocks).unwrap_or_else(|| block)));
            }
        }

        if self.join_type == "LEFT" && !self.matched_build_keys.is_empty() {
        }

        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        self.build_child.close().await?;
        self.probe_child.close().await?;
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
    pub children: Vec<Box<ExecutionPlan>>,
    pub current_child: usize,
    pub opened: Vec<bool>,
}

impl UnionExecNode {
    pub fn new(children: Vec<Box<ExecutionPlan>>) -> Self {
        let opened = vec![false; children.len()];
        Self {
            children,
            current_child: 0,
            opened,
        }
    }
}

#[async_trait]
impl ExecNode for UnionExecNode {
    async fn open(&mut self) -> Result<()> {
        for (i, child) in self.children.iter_mut().enumerate() {
            child.open().await?;
            self.opened[i] = true;
        }
        self.current_child = 0;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        while self.current_child < self.children.len() {
            if let Some(block) = self.children[self.current_child].get_next().await? {
                return Ok(Some(block));
            }
            self.current_child += 1;
        }
        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        for (child, opened) in self.children.iter_mut().zip(self.opened.iter()) {
            if *opened {
                child.close().await?;
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

#[async_trait]
impl ExecNode for TruncateExecNode {
    async fn open(&mut self) -> Result<()> {
        self.executed = false;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.executed {
            return Ok(None);
        }

        self.executed = true;

        if self.if_exists {
            tracing::info!("TRUNCATE TABLE IF EXISTS {}.{} executed", self.database, self.table_name);
        } else {
            tracing::info!("TRUNCATE TABLE {}.{}", self.database, self.table_name);
        }

        Ok(None)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct WindowExecNode {
    pub partition_by: Vec<usize>,
    pub order_by: Vec<(usize, bool)>,
    pub window_func: String,
    pub window_func_col: usize,
    pub offset: i64,
    pub default_val: ScalarValue,
    pub child: Box<ExecutionPlan>,
    pub opened: bool,
    pub returned: bool,
    pub buffered: Option<Block>,
}

impl WindowExecNode {
    pub fn new(
        window_func: String,
        window_func_col: usize,
        partition_by: Vec<usize>,
        order_by: Vec<(usize, bool)>,
        child: Box<ExecutionPlan>,
    ) -> Self {
        Self {
            partition_by,
            order_by,
            window_func,
            window_func_col,
            offset: 1,
            default_val: ScalarValue::Null,
            child,
            opened: false,
            returned: false,
            buffered: None,
        }
    }
}

#[async_trait]
impl ExecNode for WindowExecNode {
    async fn open(&mut self) -> Result<()> {
        self.child.open().await?;
        self.opened = true;
        self.returned = false;
        self.buffered = None;
        Ok(())
    }

    async fn get_next(&mut self) -> Result<Option<Block>> {
        if self.returned {
            return Ok(None);
        }

        let mut all_blocks = Vec::new();
        while let Some(block) = self.child.get_next().await? {
            all_blocks.push(block);
        }

        if all_blocks.is_empty() {
            self.returned = true;
            return Ok(None);
        }

        let combined = Block::concat(&all_blocks);
        if combined.is_none() {
            self.returned = true;
            return Ok(None);
        }
        let mut block = combined.unwrap();

        let num_rows = block.num_rows();
        let mut partition_ranges: Vec<(usize, usize)> = Vec::new();

        if self.partition_by.is_empty() {
            partition_ranges.push((0, num_rows));
        } else {
            let mut partition_start = 0;
            let mut prev_key = self.get_partition_key(&block, 0);

            for i in 1..num_rows {
                let curr_key = self.get_partition_key(&block, i);
                if curr_key != prev_key {
                    partition_ranges.push((partition_start, i));
                    partition_start = i;
                    prev_key = curr_key;
                }
            }
            partition_ranges.push((partition_start, num_rows));
        }

        let mut result_rows: Vec<Vec<ScalarValue>> = Vec::new();

        for (start, end) in partition_ranges {
            let partition_size = end - start;

            let window_values = self.compute_window_over_block_for_partition(&block, start, partition_size)?;

            for i in 0..partition_size {
                let mut result_row = Vec::new();
                for col_idx in 0..block.num_columns() {
                    if let Some(col) = block.column(col_idx) {
                        result_row.push(col.scalar_at(start + i));
                    }
                }
                let window_val = match &window_values {
                    Vector::Int64(v) => ScalarValue::Int64(*v.data().get(i).unwrap_or(&0)),
                    Vector::Int32(v) => ScalarValue::Int32(*v.data().get(i).unwrap_or(&0)),
                    Vector::Float64(v) => ScalarValue::Float64(*v.data().get(i).unwrap_or(&0.0)),
                    Vector::String(v) => ScalarValue::String(v.get(i).unwrap_or("").to_string()),
                    _ => ScalarValue::Null,
                };
                result_row.push(window_val);
                result_rows.push(result_row);
            }
        }

        if result_rows.is_empty() {
            self.returned = true;
            return Ok(None);
        }

        let num_cols = result_rows[0].len();
        let num_result_rows = result_rows.len();

        let mut columns: Vec<Vector> = Vec::new();
        for col_idx in 0..num_cols {
            let scalars: Vec<ScalarValue> = result_rows.iter()
                .filter_map(|row| row.get(col_idx).cloned())
                .collect();

            let vector = match scalars.first().unwrap_or(&ScalarValue::Null) {
                ScalarValue::Int64(_) => {
                    let data: Vec<i64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int64(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int64(types::vector::Int64Vector::from_vec(data))
                }
                ScalarValue::Int32(_) => {
                    let data: Vec<i32> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Int32(i) = s { Some(*i) } else { None }
                    }).collect();
                    Vector::Int32(types::vector::Int32Vector::from_vec(data))
                }
                ScalarValue::Float64(_) => {
                    let data: Vec<f64> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::Float64(f) = s { Some(*f) } else { None }
                    }).collect();
                    Vector::Float64(types::vector::Float64Vector::from_vec(data))
                }
                ScalarValue::String(_) => {
                    let data: Vec<String> = scalars.iter().filter_map(|s| {
                        if let ScalarValue::String(s) = s { Some(s.clone()) } else { None }
                    }).collect();
                    let data_refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
                    Vector::String(types::vector::StringVector::from_vec(data_refs))
                }
                _ => Vector::Null(types::vector::NullVector::new(num_result_rows)),
            };
            columns.push(vector);
        }

        let mut schema_fields: Vec<types::Field> = block.schema().fields().to_vec();
        schema_fields.push(types::Field::new("window_col", types::DataType::Int64, true));

        self.returned = true;
        Ok(Some(Block::new(Schema::new(schema_fields), columns)))
    }

    async fn close(&mut self) -> Result<()> {
        self.child.close().await?;
        self.opened = false;
        self.buffered = None;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WindowExecNode {
    fn get_partition_key(&self, block: &Block, row_idx: usize) -> String {
        let mut key_parts = Vec::new();
        for &col_idx in &self.partition_by {
            if col_idx < block.num_columns() {
                if let Some(col) = block.column(col_idx) {
                    key_parts.push(format!("{:?}", col.scalar_at(row_idx)));
                }
            }
        }
        key_parts.join("|")
    }

    fn compute_window_over_block(&self, block: &Block) -> Result<Vector> {
        let num_rows = block.num_rows();

        match self.window_func.as_str() {
            "row_number" => {
                let data: Vec<i64> = (1..=num_rows as i64).collect();
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            "rank" | "dense_rank" => {
                let data: Vec<i64> = (1..=num_rows as i64).collect();
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            "lead" | "lag" => {
                let mut data: Vec<i64> = Vec::new();
                if self.window_func_col < block.num_columns() {
                    if let Some(col) = block.column(self.window_func_col) {
                        for i in 0..num_rows {
                            let target_idx = if self.window_func == "lead" {
                                i + self.offset as usize
                            } else {
                                i.saturating_sub(self.offset as usize)
                            };

                            if target_idx < num_rows {
                                if let ScalarValue::Int64(v) = col.scalar_at(target_idx) {
                                    data.push(v);
                                } else {
                                    data.push(0);
                                }
                            } else {
                                if let ScalarValue::Int64(v) = self.default_val {
                                    data.push(v);
                                } else {
                                    data.push(0);
                                }
                            }
                        }
                    }
                }
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            "first_value" | "last_value" => {
                let mut data: Vec<i64> = Vec::new();
                if self.window_func_col < block.num_columns() {
                    if let Some(col) = block.column(self.window_func_col) {
                        let first_val = col.scalar_at(0);
                        let last_val = col.scalar_at(num_rows.saturating_sub(1));
                        let val = if self.window_func == "first_value" { first_val } else { last_val };
                        if let ScalarValue::Int64(v) = val {
                            data = vec![v; num_rows];
                        }
                    }
                }
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            "count" | "sum" | "avg" | "min" | "max" => {
                let data: Vec<i64> = vec![num_rows as i64; num_rows];
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            _ => {
                let data: Vec<i64> = vec![0; num_rows];
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
        }
    }

    fn compute_window_over_block_for_partition(&self, block: &Block, start: usize, size: usize) -> Result<Vector> {
        match self.window_func.as_str() {
            "row_number" => {
                let data: Vec<i64> = (1..=size as i64).collect();
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            "rank" | "dense_rank" => {
                let data: Vec<i64> = (1..=size as i64).collect();
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            "lead" | "lag" => {
                let mut data: Vec<i64> = Vec::new();
                if self.window_func_col < block.num_columns() {
                    if let Some(col) = block.column(self.window_func_col) {
                        for i in 0..size {
                            let global_idx = start + i;
                            let target_idx = if self.window_func == "lead" {
                                global_idx + self.offset as usize
                            } else {
                                global_idx.saturating_sub(self.offset as usize)
                            };

                            if target_idx >= start && target_idx < start + size {
                                if let ScalarValue::Int64(v) = col.scalar_at(target_idx) {
                                    data.push(v);
                                } else {
                                    data.push(0);
                                }
                            } else {
                                if let ScalarValue::Int64(v) = self.default_val {
                                    data.push(v);
                                } else {
                                    data.push(0);
                                }
                            }
                        }
                    }
                }
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            "first_value" | "last_value" => {
                let mut data: Vec<i64> = Vec::new();
                if self.window_func_col < block.num_columns() {
                    if let Some(col) = block.column(self.window_func_col) {
                        let first_val = col.scalar_at(start);
                        let last_val = col.scalar_at(start + size.saturating_sub(1));
                        let val = if self.window_func == "first_value" { first_val } else { last_val };
                        if let ScalarValue::Int64(v) = val {
                            data = vec![v; size];
                        }
                    }
                }
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
            _ => {
                let data: Vec<i64> = vec![0; size];
                Ok(Vector::Int64(types::vector::Int64Vector::from_vec(data)))
            }
        }
    }
}