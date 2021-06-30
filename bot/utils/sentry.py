import sentry_sdk
import subprocess

from sentry_sdk.integrations.logging import LoggingIntegration
from typing import Union

from bot.config import Sentry


def before_breadcrumb(crumb, hint):
    if not crumb["data"]["extra"]:
        crumb["data"].pop("extra")
    return crumb


def get_release() -> Union[str, None]:
    release = None
    try:
        release = (
            subprocess.Popen(
                ["git", "describe", "--abbrev=0"],
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                stdin=subprocess.DEVNULL,
            )
                .communicate()[0]
                .strip()
                .decode("utf-8")
        )
    except (OSError, IOError):
        pass

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
