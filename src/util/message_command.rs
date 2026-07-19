use tracing::warn;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::channel::Message;
use twilight_model::oauth::ApplicationIntegrationType;
use twilight_util::builder::command::CommandBuilder;

use crate::util::EmptyResult;

pub trait MessageCommand {
    const NAME: &'static str;

    const INTEGRATION_TYPES: &'static [ApplicationIntegrationType] = &[
        ApplicationIntegrationType::GuildInstall,
        ApplicationIntegrationType::UserInstall,
    ];
    const CONTEXTS: &'static [twilight_model::application::interaction::InteractionContextType] = &[
        twilight_model::application::interaction::InteractionContextType::Guild,
        twilight_model::application::interaction::InteractionContextType::BotDm,
        twilight_model::application::interaction::InteractionContextType::PrivateChannel,
    ];

    fn command() -> twilight_model::application::command::Command {
        CommandBuilder::new(Self::NAME, "", CommandType::Message)
            .integration_types(Self::INTEGRATION_TYPES.iter().copied())
            .contexts(Self::CONTEXTS.iter().copied())
            .build()
    }
}

pub fn get_message(data: &CommandData) -> EmptyResult<&Message> {
    let resolved = match &data.resolved {
        None => {
            warn!("Received Message Application Command Interaction without resolved data");
            return Err(());
        }
        Some(r) => r,
    };

    match resolved.messages.iter().next() {
        None => {
            warn!("Received Message Application Command Interaction without message");
            Err(())
        }
        Some((_, msg)) => Ok(msg),
    }
}
