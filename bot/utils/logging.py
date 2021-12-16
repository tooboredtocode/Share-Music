import logging
import sentry_sdk
import sys

from loguru import logger
from sentry_sdk.integrations.logging import (
    BreadcrumbHandler,
    EventHandler,
)
from typing import Dict, List

from bot.config import LoggingHandlers, Sentry


class LogFilter:
    def __init__(self):
        self.store: Dict[str, List[str]] = {}

    def add_filter(self, name, match):
        if not self.store.get(name):
            self.store[name] = []

        self.store[name].append(match)

    def check(self, name, message) -> bool:
        if name is None:
            return True

        if (matches := self.store.get(name)) is None:
            return False

        for match in matches:
            if match in message:
                return True

        return False


log_filter = LogFilter()


class InterceptHandler(logging.Handler):
    def emit(self, record: logging.LogRecord):
        # Get corresponding Loguru level if it exists
        try:
            level = logger.level(record.levelname).name
        except ValueError:
            level = record.levelno

        # Find the caller from where the logged message originated
        frame, depth = logging.currentframe(), 2
        while frame.f_code.co_filename in (
            logging.__file__,
            sentry_sdk.integrations.logging.__file__,
        ):
            frame = frame.f_back
            depth += 1

        # noinspection PyProtectedMember
        frame = sys._getframe(depth)
        name = frame.f_globals.get("__name__")

        message = record.getMessage()

        if log_filter.check(name, message):
            return

        logger.opt(depth=depth, exception=record.exc_info).log(level, message)


# Monkey Patch Function for Loguru to allow easy addition of extra record information
def debug(self, message, extra=None, *args, **kwargs):
    options = list(self._options)

    if extra is not None:
        static_extra = options[8]
        options[8] = {**extra, **static_extra}

    # noinspection PyProtectedMember
    logger.__class__._log(
        self, "DEBUG", None, False, tuple(options), message, args, kwargs
    )


def warning(self, message, extra=None, *args, **kwargs):
    options = list(self._options)

    if extra is not None:
        static_extra = options[8]
        options[8] = {**extra, **static_extra}

    # noinspection PyProtectedMember
    logger.__class__._log(
        self, "WARNING", None, False, tuple(options), message, args, kwargs
    )


def configure():
    # noinspection PyArgumentList
    logging.basicConfig(handlers=[InterceptHandler()], level=0)

    # monkey patching
    logger.__class__.debug = debug
    logger.__class__.warning = warning

    logger.remove()

    if Sentry.dsn:
        logger.add(
            BreadcrumbHandler(level=logging.DEBUG),
            level=logging.DEBUG,
            format="{name} - {message}",
            backtrace=False,
            diagnose=False,
        )
        logger.add(
            EventHandler(level=logging.ERROR),
            level=logging.ERROR,
            format="{name} - {message}",
            backtrace=False,
            diagnose=False,
        )

    for handler in LoggingHandlers:
        logger.add(**handler)

    log_filter.add_filter("discord.client", "Dispatching event ")
    log_filter.add_filter("discord.gateway", "WebSocket Event: ")
    log_filter.add_filter("discord.gateway", "websocket alive with sequence")
    log_filter.add_filter("discord.gateway", "Unknown event ")
    log_filter.add_filter("discord.http", "has received")
    log_filter.add_filter("discord.http", "has returned")
    log_filter.add_filter("PIL.TiffImagePlugin", "tag: ")
