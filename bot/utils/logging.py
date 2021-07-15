import asyncio
import aiohttp
import discord
import json
import logging
import sentry_sdk
import sys
import urllib

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
        while frame.f_code.co_filename in (logging.__file__, sentry_sdk.integrations.logging.__file__):
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
    logger.__class__._log(self, "DEBUG", None, False, tuple(options), message, args, kwargs)


def remove_sensitive_info(data, blacklist):
    for key, value in data.items():
        if isinstance(value, dict):
            remove_sensitive_info(value, blacklist)
        for test in blacklist:
            if key == test:
                data[key] = "redacted"


op_codes = {
    0: "",  # ignore this code so that the event can be properly displayed
    1: "Heartbeat",
    2: "Identify",
    3: "Presence Update",
    4: "Voice State Update",
    6: "Resume",
    7: "Reconnect",
    8: "Request Guild Members",
    9: "Invalid Session",
    10: "Hello",
    11: "Heartbeat ACK",
}

gateway_logger = logger.patch(lambda record: record.update(name="discord.gateway"))


def log_gateway_events(data):
    remove_sensitive_info(data, ["token"])

    key = next(iter(data))
    event = data[key].get("t")
    op_code = data[key].get("op")

    frame, depth = logging.currentframe(), 2
    while frame.f_code.co_filename in (logging.__file__, sentry_sdk.integrations.logging.__file__):
        frame = frame.f_back
        depth += 1

    gateway_logger.debug(
        f"{'Received' if key == 'gateway_in' else 'Dispatched'} gateway event "
        f"{op_codes[op_code]}{event if event else ''}",
        extra=data
    )


# Monkey Patch Functions for Discord.py to log all sent payloads
async def send_as_json(self, data):
    await discord.gateway.DiscordWebSocket.send_heartbeat_copy(self, data)
    log_gateway_events({"gateway_out": data})


async def send_heartbeat(self, data):
    await discord.gateway.DiscordWebSocket.send_as_json_copy(self, data)
    log_gateway_events({"gateway_out": data})


async def received_message(self, msg):
    if type(msg) is bytes:
        self._buffer.extend(msg)

        if len(msg) < 4 or msg[-4:] != b'\x00\x00\xff\xff':
            return
        msg = self._zlib.decompress(self._buffer)
        msg = msg.decode('utf-8')
        self._buffer = bytearray()
    data = json.loads(msg)

    log_gateway_events({"gateway_in": data})

    await discord.gateway.DiscordWebSocket.received_message_copy(self, msg)


http_logger = logger.patch(lambda record: record.update(name="discord.http"))


async def request(self, route, *, files=None, form=None, **kwargs):
    bucket = route.bucket
    method = route.method
    url = route.url

    lock = self._locks.get(bucket)
    if lock is None:
        lock = asyncio.Lock()
        if bucket is not None:
            self._locks[bucket] = lock

    # header creation
    headers = {
        'User-Agent': self.user_agent,
        'X-Ratelimit-Precision': 'millisecond',
    }

    if self.token is not None:
        headers['Authorization'] = 'Bot ' + self.token if self.bot_token else self.token
    # some checking if it's a JSON request
    if 'json' in kwargs:
        headers['Content-Type'] = 'application/json'
        kwargs['data'] = discord.utils.to_json(kwargs.pop('json'))

    try:
        reason = kwargs.pop('reason')
    except KeyError:
        pass
    else:
        if reason:
            headers['X-Audit-Log-Reason'] = urllib.parse.quote(reason, safe='/ ')

    kwargs['headers'] = headers

    # Proxy support
    if self.proxy is not None:
        kwargs['proxy'] = self.proxy
    if self.proxy_auth is not None:
        kwargs['proxy_auth'] = self.proxy_auth

    if not self._global_over.is_set():
        # wait until the global lock is complete
        await self._global_over.wait()

    await lock.acquire()
    with discord.http.MaybeUnlock(lock) as maybe_lock:
        for tries in range(5):
            if files:
                for f in files:
                    f.reset(seek=tries)

            if form:
                form_data = aiohttp.FormData()
                for params in form:
                    form_data.add_field(**params)
                kwargs['data'] = form_data

            try:
                async with self._HTTPClient__session.request(method, url, **kwargs) as r:
                    # even errors have text involved in them so this is safe to call
                    data = await discord.http.json_or_text(r)

                    http_logger.debug(
                        f"{method} {url} returned: {r.status}",
                        extra={
                            "http_out": kwargs.get("data"),
                            "http_back": data if 300 > r.status >= 200 else None
                        }
                    )

                    # check if we have rate limit header information
                    remaining = r.headers.get('X-Ratelimit-Remaining')
                    if remaining == '0' and r.status != 429:
                        # we've depleted our current bucket
                        delta = discord.utils._parse_ratelimit_header(r, use_clock=self.use_clock)
                        http_logger.debug(
                            f"A rate limit bucket has been exhausted (bucket: {bucket}, retry: {delta})."
                        )
                        maybe_lock.defer()
                        self.loop.call_later(delta, lock.release)

                    # the request was successful so just return the text/json
                    if 300 > r.status >= 200:
                        return data

                    # we are being rate limited
                    if r.status == 429:
                        if not r.headers.get('Via'):
                            # Banned by Cloudflare more than likely.
                            raise discord.errors.HTTPException(r, data)

                        # sleep a bit
                        retry_after = data['retry_after'] / 1000.0
                        http_logger.warning(
                            f"We are being rate limited. Retrying in {retry_after:.2f} "
                            f"seconds. Handled under the bucket \"{bucket}\""
                        )

                        # check if it's a global rate limit
                        is_global = data.get('global', False)
                        if is_global:
                            http_logger.warning(
                                f"Global rate limit has been hit. "
                                f"Retrying in {retry_after:.2f} seconds."
                            )
                            self._global_over.clear()

                        await asyncio.sleep(retry_after)
                        http_logger.debug('Done sleeping for the rate limit. Retrying...')

                        # release the global lock now that the
                        # global rate limit has passed
                        if is_global:
                            self._global_over.set()
                            http_logger.debug('Global rate limit is now over.')

                        continue

                    # we've received a 500 or 502, unconditional retry
                    if r.status in {500, 502}:
                        await asyncio.sleep(1 + tries * 2)
                        continue

                    # the usual error cases
                    if r.status == 403:
                        raise discord.errors.Forbidden(r, data)
                    elif r.status == 404:
                        raise discord.errors.NotFound(r, data)
                    elif r.status == 503:
                        raise discord.errors.DiscordServerError(r, data)
                    else:
                        raise discord.errors.HTTPException(r, data)

            # This is handling exceptions from the request
            except OSError as e:
                # Connection reset by peer
                if tries < 4 and e.errno in (54, 10054):
                    continue
                raise

        # We've run out of retries, raise.
        if r.status >= 500:
            raise discord.errors.DiscordServerError(r, data)

        raise discord.errors.HTTPException(r, data)


def configure():
    # noinspection PyArgumentList
    logging.basicConfig(handlers=[InterceptHandler()], level=0)

    # monkey patching
    logger.__class__.debug = debug

    discord.gateway.DiscordWebSocket.send_as_json_copy = discord.gateway.DiscordWebSocket.send_as_json
    discord.gateway.DiscordWebSocket.send_as_json = send_as_json

    discord.gateway.DiscordWebSocket.send_heartbeat_copy = discord.gateway.DiscordWebSocket.send_heartbeat
    discord.gateway.DiscordWebSocket.send_heartbeat = send_heartbeat

    discord.gateway.DiscordWebSocket.received_message_copy = discord.gateway.DiscordWebSocket.received_message
    discord.gateway.DiscordWebSocket.received_message = received_message

    discord.http.HTTPClient.request = request

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

    for handler in LoggingHandlers:
        logger.add(**handler)

    log_filter.add_filter("discord.client", "Dispatching event ")
    log_filter.add_filter("discord.gateway", "WebSocket Event: ")
    log_filter.add_filter("discord.gateway", "websocket alive with sequence")
    log_filter.add_filter("discord.gateway", "Unknown event ")
    log_filter.add_filter("discord.http", "has received")
    log_filter.add_filter("discord.http", "has returned")
