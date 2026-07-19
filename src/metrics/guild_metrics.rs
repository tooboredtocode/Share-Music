/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */
use core::fmt;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use parking_lot::RwLock;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::{MetricType, TypedMetric};
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::incoming::{GuildCreate, GuildDelete, Ready};

use crate::metrics::labels::{GuildLabels, GuildState};

#[derive(Clone)]
pub struct GuildMetrics {
    inner: Arc<RwLock<GuildMetricsInner>>,
}

#[derive(Debug)]
struct GuildMetricsInner {
    guild_states: HashMap<u32, HashMap<u64, GuildState>>,
    shard_stats: HashMap<u32, (u64, u64)>, // (available, unavailable)
}

impl fmt::Debug for GuildMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.try_read() {
            Some(lock) => f
                .debug_struct("GuildMetrics")
                .field("guild_states", &lock.guild_states)
                .field("shard_stats", &lock.shard_stats)
                .finish(),
            None => f
                .debug_struct("GuildMetrics")
                .field("guild_states", &"<locked>")
                .field("shard_stats", &"<locked>")
                .finish(),
        }
    }
}

impl GuildMetrics {
    pub fn new() -> Self {
        let inner = GuildMetricsInner {
            guild_states: Default::default(),
            shard_stats: Default::default(),
        };

        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    fn update_shard_stats(&self, shard_id: u32) {
        let lock = self.inner.read();
        if lock.guild_states.get(&shard_id).is_none() {
            return;
        };

        drop(lock); // Release read lock before acquiring write lock
        let mut lock = self.inner.write();

        let shard_store = lock
            .guild_states
            .get(&shard_id)
            .expect("Shard store should exist after read lock check");

        let mut available = 0;
        let mut unavailable = 0;

        for state in shard_store.values() {
            match state {
                GuildState::Available => available += 1,
                GuildState::Unavailable => unavailable += 1,
            }
        }

        lock.shard_stats.insert(shard_id, (available, unavailable));
    }

    pub fn register(&self, shard_id: u32, event: &Event) {
        match event {
            Event::Ready(ready) => self.register_ready(shard_id, ready),
            Event::GuildCreate(create) => self.register_create(shard_id, create.deref()),
            Event::GuildDelete(delete) => self.register_delete(shard_id, delete),
            _ => return,
        }

        self.update_shard_stats(shard_id);
    }

    fn register_ready(&self, shard_id: u32, ready: &Ready) {
        let mut lock = self.inner.write();
        let shard_store = lock.guild_states.entry(shard_id).or_default();

        for guild in &ready.guilds {
            shard_store.insert(guild.id.get(), GuildState::Unavailable);
        }
    }

    fn register_create(&self, shard_id: u32, create: &GuildCreate) {
        let mut lock = self.inner.write();
        let shard_store = lock.guild_states.entry(shard_id).or_default();

        shard_store.insert(create.id().get(), GuildState::from(create));
    }

    fn register_delete(&self, shard_id: u32, delete: &GuildDelete) {
        let mut lock = self.inner.write();
        let shard_store = lock.guild_states.entry(shard_id).or_default();

        match delete.unavailable {
            Some(true) => shard_store.insert(delete.id.get(), GuildState::Unavailable),
            _ => shard_store.remove(&delete.id.get()),
        };
    }
}

impl TypedMetric for GuildMetrics {
    const TYPE: MetricType = MetricType::Gauge;
}

impl EncodeMetric for GuildMetrics {
    fn encode(&self, mut encoder: MetricEncoder) -> Result<(), fmt::Error> {
        let lock = self.inner.read();

        for (&shard_id, &(available, unavailable)) in lock.shard_stats.iter() {
            encoder
                .encode_family(&GuildLabels {
                    shard: shard_id,
                    state: GuildState::Available,
                })?
                .encode_gauge(&available)?;

            encoder
                .encode_family(&GuildLabels {
                    shard: shard_id,
                    state: GuildState::Unavailable,
                })?
                .encode_gauge(&unavailable)?;
        }

        Ok(())
    }

    fn metric_type(&self) -> MetricType {
        Self::TYPE
    }

    fn is_empty(&self) -> bool {
        let lock = self.inner.read();
        lock.shard_stats.is_empty()
    }
}
