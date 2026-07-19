use metronomos_pulse::error::BuildDependencyError;
use metronomos_pulse::value::{ArcValue, PulseValue};
use std::cmp::max;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;
use tracing::{error, info, instrument};
use twilight_gateway::{ConfigBuilder as ShardConfigBuilder, Shard, create_iterator};
use twilight_model::channel::message::AllowedMentions;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

use crate::args::Args;
use crate::constants::cluster_consts;
use crate::util::EmptyResult;
use crate::util::error::expect_err;

#[derive(Clone, PulseValue)]
pub struct DiscordClient {
    inner: Arc<DiscordClientInner>,
}

struct DiscordClientInner {
    client: twilight_http::Client,
    bot_id: Id<ApplicationMarker>,
}

impl fmt::Debug for DiscordClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiscordClient")
            .field("client", &self.inner.client)
            .field("bot_id", &self.inner.bot_id)
            .finish()
    }
}

impl DiscordClient {
    #[instrument(name = "init_discord_client", skip_all)]
    pub async fn init(args: ArcValue<Args>) -> Result<Self, BuildDependencyError> {
        let builder = twilight_http::Client::builder()
            .token(args.token.clone())
            .default_allowed_mentions(AllowedMentions::default());

        let client = builder.build();

        info!("Validating discord api token...");

        let user = client
            .current_user()
            .await
            .map_err(expect_err!("Failed to get current user"))?
            .model()
            .await
            .map_err(expect_err!("Failed to deserialize user response"))?;

        info!(
            "Api credentials validated: {}#{} and application id {}",
            user.name,
            user.discriminator(),
            user.id
        );

        Ok(Self {
            inner: Arc::new(DiscordClientInner {
                client,
                bot_id: user.id.cast(),
            }),
        })
    }

    pub fn bot_id(&self) -> Id<ApplicationMarker> {
        self.inner.bot_id
    }

    pub fn interaction_client(&self) -> twilight_http::client::InteractionClient<'_> {
        self.inner.client.interaction(self.bot_id())
    }

    #[instrument(skip(self))]
    pub async fn create_shards(
        &self,
        cluster_id: u16,
        cluster_count: u16,
    ) -> EmptyResult<impl ExactSizeIterator<Item = Shard> + use<>> {
        if cluster_id >= cluster_count {
            error!(
                "Cluster ID ({}) must be smaller than the number of clusters ({})",
                cluster_id, cluster_count
            );
            return Err(());
        }

        let request = self.inner.client.gateway().authed();
        let response = request
            .await
            .map_err(expect_err!("Failed to get recommended number of shards"))?;
        let info = response
            .model()
            .await
            .map_err(expect_err!("Failed to get recommended number of shards"))?;

        let shard_config = ShardConfigBuilder::new(
            self.inner
                .client
                .token()
                .expect("Token should be set")
                .to_string(),
            cluster_consts::GATEWAY_INTENTS,
        )
        .presence(cluster_consts::presence())
        .build();

        let cluster_id = cluster_id as u32;
        let cluster_count = cluster_count as u32;
        let total = max(info.shards, cluster_count);

        let iter = (cluster_id..total).step_by(cluster_count as usize);

        Ok(create_iterator(iter, total, shard_config, |_, builder| {
            builder.build()
        }))
    }
}

impl Deref for DiscordClient {
    type Target = twilight_http::Client;

    fn deref(&self) -> &Self::Target {
        &self.inner.client
    }
}
