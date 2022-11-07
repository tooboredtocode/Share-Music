/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;

use parking_lot::RwLock;
use tracing::debug;
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::incoming::{GuildCreate, GuildDelete, Ready};

use crate::context::Ctx;

#[derive(Eq, PartialEq, Debug)]
pub enum GuildState {
    Available,
    Unavailable
}

impl From<bool> for GuildState {
    fn from(v: bool) -> Self {
        match v {
            true => GuildState::Unavailable,
            false => GuildState::Available
        }
    }
}

#[derive(Debug)]
pub struct GuildStore {
    inner: RwLock<HashMap<u64, HashMap<u64, GuildState>>>,
    may_resume: RwLock<HashMap<u64, HashMap<u64, GuildState>>>
}

impl GuildStore {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
            may_resume: Default::default()
        }
    }

    pub fn count(&self, shard_id: u64, matching: Option<GuildState>) -> u64 {
        let lock = self.inner.read();

        let stats = match lock.get(&shard_id) {
            Some(m) => m,
            None => return 0
        };

        match matching {
            Some(s) => stats.iter().filter(|e| e.1 == &s).count() as u64,
            None => stats.len() as u64
        }
    }

    pub fn register(&self, shard_id: u64, event: &Event, ctx: &Ctx) {
        match event {
            Event::Ready(ready) => self.register_ready(shard_id, ready.deref()),
            Event::Resumed => self.register_resume(shard_id),
            Event::GuildCreate(create) => self.register_create(shard_id, create.deref()),
            Event::GuildDelete(delete) => self.register_delete(shard_id, delete.deref()),
            Event::ShardDisconnected(_) => self.register_disconnect(shard_id, ctx),
            _ => return
        }

        ctx.metrics.connected_guilds
            .get_metric_with_label_values(&[&shard_id.to_string(), "available"])
            .unwrap()
            .set(self.count(shard_id, Some(GuildState::Available)) as i64);
        ctx.metrics.connected_guilds
            .get_metric_with_label_values(&[&shard_id.to_string(), "unavailable"])
            .unwrap()
            .set(self.count(shard_id, Some(GuildState::Unavailable)) as i64);
    }

    fn register_ready(&self, shard_id: u64, ready: &Ready) {
        let mut lock = self.inner.write();
        let shard_store = lock
            .entry(shard_id)
            .or_insert(Default::default());

        for guild in &ready.guilds {
            shard_store.insert(guild.id.get(), GuildState::Unavailable);
        }

        let mut lock = self.may_resume.write();
        lock.remove(&shard_id);
    }

    fn register_resume(&self, shard_id: u64) {
        let mut lock = self.may_resume.write();
        if let Some(stored) = lock.remove(&shard_id) {
            let mut lock = self.inner.write();
            lock.insert(shard_id, stored);
        }
    }

    fn register_create(&self, shard_id: u64, create: &GuildCreate) {
        let mut lock = self.inner.write();
        let shard_store = lock
            .entry(shard_id)
            .or_insert(Default::default());

        shard_store.insert(create.id.get(), GuildState::from(create.unavailable));
    }

    fn register_delete(&self, shard_id: u64, delete: &GuildDelete) {
        let mut lock = self.inner.write();
        let shard_store = lock
            .entry(shard_id)
            .or_insert(Default::default());

        match delete.unavailable {
            true => shard_store.insert(delete.id.get(), GuildState::Unavailable),
            false => shard_store.remove(&delete.id.get())
        };
    }

    fn register_disconnect(&self, shard_id: u64, ctx: &Ctx) {
        let mut lock = self.inner.write();
        if let Some(s) = lock.remove(&shard_id) {
            let mut lock = self.may_resume.write();
            lock.insert(shard_id, s);

            let context_clone = ctx.clone();
            tokio::spawn(async move {
                debug!("Starting Shard Info Cleanup Thread");
                tokio::time::sleep(Duration::from_secs(60 * 5)).await;

                let mut lock = context_clone.metrics.guild_store.may_resume.write();
                lock.remove(&shard_id);
            });
        }
    }
}
