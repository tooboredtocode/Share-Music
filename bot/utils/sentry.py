import os
import sentry_sdk
import subprocess

from sentry_sdk.integrations.logging import LoggingIntegration
from typing import Union

from bot.config import Sentry


def before_breadcrumb(crumb, hint):
    data = crumb.get("data")

    if not data:
        return crumb

    if not data.get("extra"):
        crumb["data"].pop("extra")

    return crumb


def get_release() -> Union[str, None]:
    release = os.environ.get("VERSION")
    if release == "":
        release = "dev"

    return f"share-music@{release}"


def configure():
    sentry_sdk.init(
        dsn=Sentry.dsn,
        release=get_release(),
        before_breadcrumb=before_breadcrumb,
        integrations=[
            LoggingIntegration(level=None, event_level=None)
        ]
    )
