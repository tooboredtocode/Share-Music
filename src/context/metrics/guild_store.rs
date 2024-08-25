/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::collections::HashMap;
use std::ops::Deref;

use parking_lot::RwLock;
use prometheus_client::encoding::EncodeLabelValue;
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::incoming::{GuildCreate, GuildDelete, Ready};

use crate::context::metrics::GuildLabels;
use crate::context::Ctx;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum GuildState {
    Available,
    Unavailable,
}

impl From<bool> for GuildState {
    fn from(v: bool) -> Self {
        match v {
            true => GuildState::Unavailable,
            false => GuildState::Available,
        }
    }
}

impl GuildState {
    fn iter() -> impl Iterator<Item = GuildState> {
        [GuildState::Available, GuildState::Unavailable].into_iter()
    }
}

#[derive(Debug)]
pub struct GuildStore {
    inner: RwLock<HashMap<u64, HashMap<u64, GuildState>>>,
}

impl GuildStore {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn count(&self, shard_id: u64, matching: Option<GuildState>) -> u64 {
        let lock = self.inner.read();

        let stats = match lock.get(&shard_id) {
            Some(m) => m,
            None => return 0,
        };

        match matching {
            Some(s) => stats.iter().filter(|e| e.1 == &s).count() as u64,
            None => stats.len() as u64,
        }
    }

    pub fn register(&self, shard_id: u64, event: &Event, ctx: &Ctx) {
        match event {
            Event::Ready(ready) => self.register_ready(shard_id, ready.deref()),
            Event::GuildCreate(create) => self.register_create(shard_id, create.deref()),
            Event::GuildDelete(delete) => self.register_delete(shard_id, delete),
            _ => return,
        }

        for state in GuildState::iter() {
            ctx.metrics
                .connected_guilds
                .get_or_create(&GuildLabels {
                    shard: shard_id,
                    state,
                })
                .set(self.count(shard_id, Some(state)) as i64);
        }
    }

    fn register_ready(&self, shard_id: u64, ready: &Ready) {
        let mut lock = self.inner.write();
        let shard_store = lock.entry(shard_id).or_insert(Default::default());

        for guild in &ready.guilds {
            shard_store.insert(guild.id.get(), GuildState::Unavailable);
        }
    }

    fn register_create(&self, shard_id: u64, create: &GuildCreate) {
        let mut lock = self.inner.write();
        let shard_store = lock.entry(shard_id).or_insert(Default::default());

        shard_store.insert(create.id.get(), GuildState::from(create.unavailable));
    }

    fn register_delete(&self, shard_id: u64, delete: &GuildDelete) {
        let mut lock = self.inner.write();
        let shard_store = lock.entry(shard_id).or_insert(Default::default());

        match delete.unavailable {
            true => shard_store.insert(delete.id.get(), GuildState::Unavailable),
            false => shard_store.remove(&delete.id.get()),
        };
    }
}
