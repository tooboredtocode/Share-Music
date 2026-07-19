use std::time::Duration;

use metronomos::builder::RuntimeBuilder;
use metronomos_pulse::builder::ProvideError;
use metronomos_pulse::error::BuildDependencyError;
use tracing::instrument;

use crate::util::error::expect_err;

pub mod colour;
pub mod discord;
pub mod odesli;

#[instrument(skip_all)]
pub fn init_http_client() -> Result<reqwest::Client, BuildDependencyError> {
    let client = reqwest::Client::builder()
        .user_agent(crate::constants::USER_AGENT)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(expect_err!("Failed to create HTTP client"))?;

    Ok(client)
}

pub fn provide_clients(b: &mut RuntimeBuilder) -> Result<(), ProvideError> {
    b.provide(init_http_client)?;
    b.provide_async(discord::DiscordClient::init)?;

    b.provide(colour::ImageClient::init)?;
    b.provide(odesli::OdesliClient::init)?;

    Ok(())
}
