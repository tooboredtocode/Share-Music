/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use twilight_interactions::command::{CommandModel, CreateCommand, DescLocalizations};

use crate::util::discord_locales::DiscordLocale;

fn share_desc() -> DescLocalizations {
    DescLocalizations::new(
        "Share Music to all Platforms",
        [(
            DiscordLocale::German.to_str(),
            "Teile Musik von für alle Plattformen",
        )],
    )
}

fn url_desc_localizations() -> DescLocalizations {
    DescLocalizations::new(
        "The Link for the Song/Album",
        [(
            DiscordLocale::German.to_str(),
            "Der Link von dem Song/Album",
        )],
    )
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "share",
    desc_localizations = "share_desc",
    integration_types = "guild_install user_install",
    contexts = "guild bot_dm private_channel"
)]
pub struct ShareCommand {
    #[command(desc_localizations = "url_desc_localizations")]
    pub url: String,
}
