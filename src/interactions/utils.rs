use std::future::pending;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time;
use tracing::{Instrument, debug_span, warn};
use twilight_interactions::command::CommandModel;
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::interactions::InteractionsHandler;
use crate::util::EmptyResult;
use crate::util::error::expect_warn;

impl InteractionsHandler {
    pub fn parse_command<C: CommandModel>(&self, data: CommandData) -> EmptyResult<C> {
        match C::from_interaction(data.into()).map_err(expect_warn!(
            "Received Invalid Interaction data, re-syncing commands",
        )) {
            Ok(s) => Ok(s),
            Err(_) => {
                let this = self.clone();
                tokio::spawn(async move {
                    // TODO: Shutdown on failure to sync commands, as this is a critical error.
                    this.sync_commands().await
                });
                Err(())
            }
        }
    }

    pub fn defer(&self, inter: &Interaction) -> JoinHandle<()> {
        let inter_id = inter.id;
        let inter_token = inter.token.clone();
        let this = self.clone();

        tokio::spawn(
            async move {
                if let Err(e) = this
                    .discord()
                    .interaction_client()
                    .create_response(
                        inter_id,
                        inter_token.as_str(),
                        &InteractionResponse {
                            kind: InteractionResponseType::DeferredChannelMessageWithSource,
                            data: None,
                        },
                    )
                    .await
                {
                    warn!("Failed to defer Response, this may cause the Interaction to fail: {e}");
                }
            }
            .instrument(debug_span!("deferring_response")),
        )
    }

    pub async fn respond_with(&self, inter: &Interaction, msg: &str) {
        let _ = self
            .discord()
            .interaction_client()
            .create_response(
                inter.id,
                inter.token.as_str(),
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: InteractionResponseDataBuilder::new()
                        .content(msg)
                        .flags(MessageFlags::EPHEMERAL)
                        .build()
                        .into(),
                },
            )
            .await
            .map_err(expect_warn!("Failed to respond to the Interaction"));
    }

    pub async fn update_defer_with_error(&self, inter: &Interaction, msg: &str) {
        if self
            .discord()
            .interaction_client()
            .update_response(inter.token.as_str())
            .content(Some(msg))
            .into_future()
            .instrument(debug_span!("sending_error_message"))
            .await
            .map_err(expect_warn!("Failed to inform user of the error"))
            .is_ok()
        {
            let this = self.clone();
            let inter_token = inter.token.clone();
            tokio::spawn(async move {
                let _ = time::timeout(
                    Duration::from_secs(15),
                    pending::<()>(), // TODO: Sync this with runtime shutdown.
                )
                .await;

                this.discord()
                    .interaction_client()
                    .delete_response(inter_token.as_str())
                    .into_future()
                    .instrument(debug_span!("deleting_error_message"))
                    .await
                    .map_err(expect_warn!("Failed to delete Error Message"))
            });
        }
    }
}
