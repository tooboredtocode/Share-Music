use std::fmt;
use std::sync::Arc;

use metronomos_pulse::error::BuildDependencyError;
use metronomos_pulse::value::{ArcValue, PulseValue};
use tracing::instrument;
use twilight_model::application::interaction::Interaction;
use twilight_model::application::interaction::application_command::CommandData;

use crate::args::Args;
use crate::clients::colour::ImageClient;
use crate::clients::discord::DiscordClient;
use crate::clients::odesli::OdesliClient;
use crate::db::Database;
use crate::util::error::ExpectErr;

mod commands;
mod handlers;
mod utils;

#[derive(Clone, PulseValue)]
pub struct InteractionsHandler {
    inner: Arc<InteractionsHandlerInner>,
}

#[derive(Debug)]
struct InteractionsHandlerInner {
    args: ArcValue<Args>,
    db: Database,
    discord: DiscordClient,
    odesli: OdesliClient,
    image: ImageClient,
}

impl fmt::Debug for InteractionsHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InteractionsHandler")
            .field("args", &"<args>")
            .field("db", &self.inner.db)
            .field("discord", &self.inner.discord)
            .field("odesli", &self.inner.odesli)
            .field("image", &self.inner.image)
            .finish()
    }
}

impl InteractionsHandler {
    #[instrument(name = "init_interactions_handler", skip_all)]
    pub async fn init(
        args: ArcValue<Args>,
        db: Database,
        discord: DiscordClient,
        odesli: OdesliClient,
        image: ImageClient,
    ) -> Result<Self, BuildDependencyError> {
        let inner = InteractionsHandlerInner {
            args,
            db,
            discord,
            odesli,
            image,
        };

        let res = Self {
            inner: Arc::new(inner),
        };

        res.sync_commands().await.map_err(|_| ExpectErr)?;

        Ok(res)
    }

    #[inline]
    fn args(&self) -> &Args {
        &self.inner.args
    }

    #[inline]
    fn db(&self) -> &Database {
        &self.inner.db
    }

    #[inline]
    fn discord(&self) -> &DiscordClient {
        &self.inner.discord
    }

    #[inline]
    fn odesli(&self) -> &OdesliClient {
        &self.inner.odesli
    }

    #[inline]
    fn image(&self) -> &ImageClient {
        &self.inner.image
    }
}
