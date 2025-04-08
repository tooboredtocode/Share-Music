/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use tracing::info;
use twilight_http::client::InteractionClient;
use twilight_http::Client;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use crate::context::Context;
use crate::util::EmptyResult;
use crate::util::error::expect_err;

impl Context {
    pub(super) async fn discord_client_from_config(
        token: &str,
    ) -> EmptyResult<(Client, Id<ApplicationMarker>)> {
        let builder = Client::builder()
            .token(token.to_string())
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

        Ok((client, Id::new(user.id.get())))
    }

    pub fn interaction_client(&self) -> InteractionClient<'_> {
        self.discord_client.interaction(self.bot_id)
    }
}
