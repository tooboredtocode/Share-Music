/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

pub const NAME: &str = "ShareMusic";
pub const NAME_SHORT: &str = "Sharing";

pub const USER_AGENT: &str = concat!("share-music-bot/", env!("CARGO_PKG_VERSION"));

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const RUST_VERSION: &str = env!("BUILD_RUSTC_VERSION");
pub const GIT_BRANCH: &str = env!("BUILD_GIT_BRANCH");
pub const GIT_REVISION: &str = env!("BUILD_GIT_REVISION");

pub mod cluster_consts {
    use twilight_model::gateway::Intents;
    use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
    use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};

    pub const GATEWAY_INTENTS: Intents = Intents::GUILDS;

    pub fn presence() -> UpdatePresencePayload {
        UpdatePresencePayload {
            activities: vec![
                MinimalActivity {
                    kind: ActivityType::Listening,
                    name: "your requests!".to_string(),
                    url: None,
                }
                .into(),
            ],
            afk: false,
            since: None,
            status: Status::Online,
        }
    }
}

pub mod colour_consts {
    pub const MAX_IMAGE_SIZE: u32 = 4096;
}
