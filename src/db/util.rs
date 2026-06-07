/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use twilight_model::id::Id;

pub(super) fn snowflake_to_db<T>(id: Id<T>) -> i64 {
    id.get() as i64
}
