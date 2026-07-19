use std::collections::HashMap;
use std::fmt::Error;
use std::sync::Arc;

use parking_lot::Mutex;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::{MetricType, TypedMetric};
use twilight_gateway::ShardState;

use crate::metrics::labels::ShardStateLabels;

#[derive(Clone, Debug)]
pub struct ShardStates {
    inner: Arc<Mutex<HashMap<u32, ShardState>>>,
}

impl ShardStates {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn shard_status_to_str(status: ShardState) -> &'static str {
        use ShardState::*;

        match status {
            Active => "Active",
            Disconnected { .. } => "Disconnected",
            FatallyClosed => "FatallyClosed",
            Identifying => "Identifying",
            Resuming => "Resuming",
        }
    }

    fn shard_state_iter() -> impl Iterator<Item = ShardState> {
        [
            ShardState::Active,
            ShardState::Disconnected {
                reconnect_attempts: 0,
            },
            ShardState::FatallyClosed,
            ShardState::Identifying,
            ShardState::Resuming,
        ]
        .into_iter()
    }

    fn loose_state_match(state1: ShardState, state2: ShardState) -> bool {
        match (state1, state2) {
            (ShardState::Disconnected { .. }, ShardState::Disconnected { .. }) => true,
            _ => state1 == state2,
        }
    }

    pub fn update_shard_state(&self, shard_id: u32, state: ShardState) {
        let mut lock = self.inner.lock();
        lock.insert(shard_id, state);
    }
}

impl TypedMetric for ShardStates {
    const TYPE: MetricType = MetricType::Gauge;
}

impl EncodeMetric for ShardStates {
    fn encode(&self, mut encoder: MetricEncoder) -> Result<(), Error> {
        let lock = self.inner.lock();

        for (shard_id, &shard_state) in lock.iter() {
            for state in Self::shard_state_iter() {
                let value = match Self::loose_state_match(shard_state, state) {
                    true => 1u32,
                    false => 0u32,
                };

                encoder
                    .encode_family(&ShardStateLabels {
                        shard: *shard_id,
                        state: Self::shard_status_to_str(state),
                    })?
                    .encode_gauge(&value)?;
            }
        }

        Ok(())
    }

    fn metric_type(&self) -> MetricType {
        Self::TYPE
    }

    fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }
}
