import logging
import sentry_sdk

from loguru import logger
from sentry_sdk.integrations.logging import (
    BreadcrumbHandler,
    EventHandler
)

from bot.config import LoggingConfigs, Sentry


class InterceptHandler(logging.Handler):
    def emit(self, record):
        # Get corresponding Loguru level if it exists
        try:
            level = logger.level(record.levelname).name
        except ValueError:
            level = record.levelno

        # Find the caller from where the logged message originated
        frame, depth = logging.currentframe(), 2
        while frame.f_code.co_filename in (logging.__file__, sentry_sdk.integrations.logging.__file__):
            frame = frame.f_back
            depth += 1

        logger.opt(depth=depth, exception=record.exc_info).log(level, record.getMessage())


def configure():
    # noinspection PyArgumentList
    logging.basicConfig(handlers=[InterceptHandler()], level=0)

    logger.remove()

    if Sentry.dsn:
        logger.add(
            BreadcrumbHandler(level=logging.DEBUG),
            level=logging.DEBUG,
            format="{name} - {message}",
            backtrace=False,
            diagnose=False
        )
        logger.add(
            EventHandler(level=logging.ERROR),
            level=logging.ERROR,
            format="{name} - {message}",
            backtrace=False,
            diagnose=False
        )

    for config in LoggingConfigs:
        logger.add(**config)
