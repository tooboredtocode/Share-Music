use std::mem;
use tracing::debug;
use twilight_interactions::command::CreateCommand;
use twilight_model::application::interaction::InteractionData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::channel::message::component::ComponentType;

use crate::interactions::commands::find_links::FindLinksCommand;
use crate::interactions::commands::share::ShareCommand;
use crate::interactions::commands::test_colour_consts::TestColorConstsCommand;
use crate::interactions::{CommandData, Interaction, InteractionsHandler, instrument};
use crate::util::message_command::MessageCommand;

mod common;
mod find_links;
mod messages;
mod share;
mod show_player;
mod test_colour_consts;

impl InteractionsHandler {
    #[instrument(
        name = "interaction_handler",
        level = "debug",
        skip_all,
        fields(
            inter_id = inter.id.get(),
            user_id = inter.author_id().map(|id| id.get()),
            channel_id = inter.channel.as_ref().map(|channel| channel.id.get()),
            guild_id = inter.guild_id.map(|id| id.get())
        )
    )]
    pub async fn handle(&self, mut inter: Interaction) {
        match mem::take(&mut inter.data) {
            Some(InteractionData::ApplicationCommand(command_data)) => {
                self.handle_application_commands(inter, *command_data).await;
            }
            Some(InteractionData::MessageComponent(component_data)) => {
                self.handle_message_components(inter, *component_data).await;
            }
            _ => {
                debug!("Received Unexpected {} Interaction", inter.kind.kind());
            }
        }
    }

    async fn handle_application_commands(&self, inter: Interaction, command_data: CommandData) {
        debug!("Received Application Command Interaction");

        match command_data.name.as_str() {
            ShareCommand::NAME => {
                self.handle_share(inter, command_data).await;
            }
            TestColorConstsCommand::NAME => {
                self.handle_test_colour_consts(inter, command_data).await;
            }
            FindLinksCommand::NAME => self.handle_find_links(inter, command_data).await,
            name => debug!(
                "Unknown {} Application Command Interaction: {}",
                command_data.kind.kind(),
                name
            ),
        }
    }

    async fn handle_message_components(
        &self,
        inter: Interaction,
        component_data: MessageComponentInteractionData,
    ) {
        debug!("Received Message Component Interaction");

        if component_data.component_type == ComponentType::TextSelectMenu {
            debug!(
                "Received Text Select Menu Interaction with custom_id: {}",
                component_data.custom_id
            );
            // Handle the Show Player Select Menu Interaction if the custom_id starts with the expected prefix
            if component_data.custom_id.starts_with(show_player::SELECT_ID) {
                debug!("Handling Show Player Select Menu Interaction");
                return self.handle_show_player(inter, component_data).await;
            }
        }

        debug!(
            "Unknown {} Application Command Interaction: {}",
            component_data.component_type.name(),
            component_data.custom_id
        );
    }
}
