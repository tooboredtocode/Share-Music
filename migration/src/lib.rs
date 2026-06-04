/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

pub use sea_orm_migration::prelude::*;

mod m20260604_01_create_usage_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260604_01_create_usage_table::Migration)]
    }
}
