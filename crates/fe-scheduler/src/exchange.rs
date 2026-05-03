use common::DrorisError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use types::Block;

use crate::cluster::NodeAddress;

// ---------------------------------------------------------------------------
// Exchange type descriptors
// ---------------------------------------------------------------------------

/// Describes how data flows between fragment instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExchangeKind {
    /// Partition rows by hashing key columns into `num_partitions` channels.
    HashPartition {
        /// Column indices used as the hash key.
        key_columns: Vec<usize>,
        /// Number of output partitions (channels).
        num_partitions: usize,
    },
    /// Send every row to every consumer instance.
    Broadcast,
    /// Collect from all producer instances into a single consumer.
    Gather,
}

/// Identifies a specific channel within an exchange (producer partition -> consumer slot).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId {
    pub fragment_instance_id: String,
    pub partition: usize,
}

/// Describes the destination where a producer sends its output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeDestination {
    /// The target node address.
    pub node: NodeAddress,
    /// The fragment instance ID on the consumer side.
    pub target_instance_id: String,
    /// Which partition / channel this destination corresponds to.
    pub channel: ChannelId,
}

// ---------------------------------------------------------------------------
// ExchangeSink: sends blocks from a producer to remote BE nodes
// ---------------------------------------------------------------------------

/// Sink that routes outgoing blocks to the correct remote destinations based
/// on the exchange kind.
#[derive(Debug, Clone)]
pub struct ExchangeSink {
    pub kind: ExchangeKind,
    pub destinations: Vec<ExchangeDestination>,
    /// Buffered blocks per channel (partition key -> accumulated blocks).
    buffers: HashMap<usize, Vec<Block>>,
}

impl ExchangeSink {
    pub fn new(kind: ExchangeKind, destinations: Vec<ExchangeDestination>) -> Self {
        Self {
            kind,
            destinations,
            buffers: HashMap::new(),
        }
    }

    /// Route a block to the correct channel buffer(s) based on the exchange kind.
    /// For HashPartition the block is split; for Broadcast it is replicated;
    /// for Gather it goes to channel 0.
    pub fn send(&mut self, block: Block) -> Result<(), DrorisError> {
        match &self.kind {
            ExchangeKind::HashPartition {
                key_columns,
                num_partitions,
            } => {
                // Partition the block by hashing key columns.
                // Simplified: use row-level hash modulo num_partitions.
                let partitions = self.hash_partition_block(&block, key_columns, *num_partitions);
                for (part_idx, part_block) in partitions.into_iter().enumerate() {
                    if part_block.num_rows() > 0 {
                        self.buffers
                            .entry(part_idx)
                            .or_default()
                            .push(part_block);
                    }
                }
                Ok(())
            }
            ExchangeKind::Broadcast => {
                // Send to every destination.
                self.buffers
                    .entry(0)
                    .or_default()
                    .push(block);
                Ok(())
            }
            ExchangeKind::Gather => {
                self.buffers
                    .entry(0)
                    .or_default()
                    .push(block);
                Ok(())
            }
        }
    }

    /// Flush all buffered blocks to the remote destinations.
    /// In a real implementation this would serialize and send over RPC.
    /// Here we return the batches per destination for the caller to dispatch.
    pub fn flush(&mut self) -> Vec<(ExchangeDestination, Vec<Block>)> {
        let mut results = Vec::new();

        match &self.kind {
            ExchangeKind::HashPartition { num_partitions, .. } => {
                for dest in &self.destinations {
                    let part = dest.channel.partition;
                    if let Some(blocks) = self.buffers.remove(&part) {
                        results.push((dest.clone(), blocks));
                    }
                }
                // Remaining partitions not matched to a specific destination.
                for (part, blocks) in self.buffers.drain() {
                    if part < *num_partitions {
                        // Find the matching destination or create a synthetic one.
                        if let Some(dest) = self.destinations.iter().find(|d| d.channel.partition == part) {
                            results.push((dest.clone(), blocks));
                        }
                    }
                }
            }
            ExchangeKind::Broadcast => {
                if let Some(blocks) = self.buffers.remove(&0) {
                    for dest in self.destinations.clone() {
                        results.push((dest, blocks.clone()));
                    }
                }
            }
            ExchangeKind::Gather => {
                if let Some(blocks) = self.buffers.remove(&0) {
                    if let Some(dest) = self.destinations.first().cloned() {
                        results.push((dest, blocks));
                    }
                }
            }
        }

        self.buffers.clear();
        results
    }

    /// Hash-partition a block into `num_partitions` sub-blocks.
    /// Uses a simple hash of the key column values.
    fn hash_partition_block(
        &self,
        block: &Block,
        key_columns: &[usize],
        num_partitions: usize,
    ) -> Vec<Block> {
        if num_partitions == 0 || block.num_rows() == 0 {
            return vec![];
        }

        let mut row_assignments: Vec<usize> = vec![0; block.num_rows()];
        for row_idx in 0..block.num_rows() {
            let mut hash: u64 = 0;
            for &col_idx in key_columns {
                if let Some(col) = block.column(col_idx) {
                    let scalar = col.scalar_at(row_idx);
                    hash = hash.wrapping_add(Self::hash_scalar(&scalar));
                }
            }
            row_assignments[row_idx] = (hash % num_partitions as u64) as usize;
        }

        // Build per-partition row indices and slice.
        let mut partitions = Vec::with_capacity(num_partitions);
        for part in 0..num_partitions {
            let indices: Vec<usize> = row_assignments
                .iter()
                .enumerate()
                .filter(|&(_, p)| p == &part)
                .map(|(i, _)| i)
                .collect();

            if indices.is_empty() {
                partitions.push(Block::empty(block.schema().clone()));
            } else {
                // Collect rows for this partition. Simplified: slice contiguous runs.
                // A production implementation would build a bitmap and use Block::filter.
                let mut part_block = block.slice(indices[0], 1);
                for &idx in &indices[1..] {
                    let row_block = block.slice(idx, 1);
                    part_block.append_block(&row_block);
                }
                partitions.push(part_block);
            }
        }
        partitions
    }

    /// Simple FNV-1a-inspired hash for scalar values.
    fn hash_scalar(val: &types::ScalarValue) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        // We use a simple string-based hashing approach.
        let mut hasher = DefaultHasher::new();
        format!("{:?}", val).hash(&mut hasher);
        hasher.finish()
    }

    /// Total number of buffered rows across all channels.
    pub fn buffered_rows(&self) -> usize {
        self.buffers
            .values()
            .flat_map(|blocks| blocks.iter().map(|b| b.num_rows()))
            .sum()
    }
}

// ---------------------------------------------------------------------------
// ExchangeSource: receives blocks from remote BE nodes
// ---------------------------------------------------------------------------

/// Source that collects incoming blocks from one or more remote producers.
#[derive(Debug)]
pub struct ExchangeSource {
    /// The fragment instance ID that this source feeds into.
    pub consumer_instance_id: String,
    /// Channels we expect to receive data from.
    pub expected_channels: Vec<ChannelId>,
    /// Received blocks, buffered per source channel.
    received: HashMap<ChannelId, Vec<Block>>,
    /// Whether each channel has signaled end-of-stream.
    finished: HashMap<ChannelId, bool>,
}

impl ExchangeSource {
    pub fn new(consumer_instance_id: String, expected_channels: Vec<ChannelId>) -> Self {
        let finished = expected_channels
            .iter()
            .map(|ch| (ch.clone(), false))
            .collect();
        Self {
            consumer_instance_id,
            expected_channels,
            received: HashMap::new(),
            finished,
        }
    }

    /// Accept a block arriving on a specific channel.
    pub fn receive(&mut self, channel: &ChannelId, block: Block) -> Result<(), DrorisError> {
        if !self.expected_channels.contains(channel) {
            return Err(DrorisError::Internal(format!(
                "unexpected channel {:?} for instance {}",
                channel, self.consumer_instance_id
            )));
        }
        self.received
            .entry(channel.clone())
            .or_default()
            .push(block);
        Ok(())
    }

    /// Mark a channel as finished (no more data).
    pub fn finish_channel(&mut self, channel: &ChannelId) {
        if let Some(done) = self.finished.get_mut(channel) {
            *done = true;
        }
    }

    /// Returns true when all expected channels have finished sending data.
    pub fn is_complete(&self) -> bool {
        self.finished.values().all(|&done| done)
    }

    /// Collect all received blocks across all channels, concatenating them.
    /// Returns None if no data has been received.
    pub fn collect(&mut self) -> Option<Block> {
        let all_blocks: Vec<Block> = self
            .received
            .values_mut()
            .flat_map(|blocks| blocks.drain(..))
            .collect();
        Block::concat(&all_blocks)
    }

    /// Total rows received so far.
    pub fn received_rows(&self) -> usize {
        self.received
            .values()
            .flat_map(|blocks| blocks.iter().map(|b| b.num_rows()))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gather_exchange_sink() {
        let dest = ExchangeDestination {
            node: NodeAddress {
                host: "127.0.0.1".into(),
                rpc_port: 9060,
                http_port: 8060,
            },
            target_instance_id: "inst-0".into(),
            channel: ChannelId {
                fragment_instance_id: "frag-0".into(),
                partition: 0,
            },
        };
        let mut sink = ExchangeSink::new(ExchangeKind::Gather, vec![dest]);
        assert_eq!(sink.buffered_rows(), 0);
    }
}
