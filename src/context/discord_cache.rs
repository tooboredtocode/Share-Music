/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use twilight_cache_inmemory::InMemoryCache;

use crate::constants::cluster_consts;
use crate::Context;

impl Context {
    pub(super) fn create_discord_cache() -> InMemoryCache {
        InMemoryCache::builder()
            .resource_types(cluster_consts::CACHED_TYPES)
            .build()
    }
}