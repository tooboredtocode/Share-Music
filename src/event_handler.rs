use crate::db::{Database, GuildMetadata, UserMetadata};
use crate::interactions::InteractionsHandler;
use crate::metrics::MetricsStore;
use metronomos_pulse::value::PulseValue;
use twilight_gateway::Shard;
use twilight_model::gateway::event::Event;

#[derive(Debug, Clone, PulseValue)]
pub struct EventHandler {
    interactions_handler: InteractionsHandler,
    database: Database,
    metrics: MetricsStore,
}

impl EventHandler {
    pub fn init(
        interactions_handler: InteractionsHandler,
        database: Database,
        metrics: MetricsStore,
    ) -> Self {
        Self {
            interactions_handler,
            database,
            metrics,
        }
    }

    pub fn handle_event(&self, shard: &Shard, event: Event) {
        self.metrics.update_cluster_metrics(shard, &event);

        self.persist_guild_metadata(&event);
        self.persist_user_metadata(&event);

        let handler = self.interactions_handler.clone();
        tokio::spawn(async move {
            #[allow(clippy::single_match)]
            match event {
                Event::InteractionCreate(event) => handler.handle(event.0).await,
                _ => {}
            }
        });
    }

    fn persist_guild_metadata(&self, event: &Event) {
        let Some(metadata) = GuildMetadata::try_from_event(event) else {
            return; // Not a guild-related event, or guild is unavailable.
        };

        // Spawn a task to save the metadata to the database asynchronously.
        self.database.spawn_save_to_db(metadata);
    }

    fn persist_user_metadata(&self, event: &Event) {
        let Some(metadata) = UserMetadata::try_from_event(event) else {
            return; // Not an event containing user information
        };

        // Spawn a task to save the metadata to the database asynchronously.
        self.database.spawn_save_to_db(metadata);
    }
}
