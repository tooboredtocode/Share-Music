import sentry_sdk

from sentry_sdk.integrations.logging import LoggingIntegration

from bot.config import Sentry


def before_breadcrumb(crumb, hint):
    if not crumb["data"]["extra"]:
        crumb["data"].pop("extra")
    return crumb


def configure():
    sentry_sdk.init(
        dsn=Sentry.dsn,
        before_breadcrumb=before_breadcrumb,
        integrations=[
            LoggingIntegration(level=None, event_level=None)
        ]
    )
