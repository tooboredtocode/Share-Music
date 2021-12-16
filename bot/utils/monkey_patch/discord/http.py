import asyncio
import aiohttp
import discord
import urllib

from loguru import logger

from bot.config import Metrics as MetricsConf
from bot.utils.metrics import api_histogram, Timer

http_logger = logger.patch(lambda record: record.update(name="discord.http"))


def sanitize_url(url: str) -> str:
    path_components = url.split("/")

    if path_components[5] == "webhooks":
        path_components[7] = "--token--"
    if path_components[5] == "interactions":
        path_components[7] = "--token--"
    if len(path_components) > 9:
        if path_components[9] == "reactions":
            path_components[10] = "--id--"

    return "/".join(path_components)


def remove_ids(url: str) -> str:
    path_components = url.split("/")

    for index, component in enumerate(path_components):
        if component.isdigit():
            path_components[index] = "--id--"

    return "/".join(path_components)


IGNORE_PATH = f"channels/{MetricsConf.channel}"


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
        "User-Agent": self.user_agent,
        "X-Ratelimit-Precision": "millisecond",
    }

    if self.token is not None:
        headers["Authorization"] = "Bot " + self.token if self.bot_token else self.token
    # some checking if it's a JSON request
    if "json" in kwargs:
        headers["Content-Type"] = "application/json"
        kwargs["data"] = discord.utils.to_json(kwargs.pop("json"))

    try:
        reason = kwargs.pop("reason")
    except KeyError:
        pass
    else:
        if reason:
            headers["X-Audit-Log-Reason"] = urllib.parse.quote(reason, safe="/ ")

    kwargs["headers"] = headers

    # Proxy support
    if self.proxy is not None:
        kwargs["proxy"] = self.proxy
    if self.proxy_auth is not None:
        kwargs["proxy_auth"] = self.proxy_auth

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
                kwargs["data"] = form_data

            try:
                timer = Timer()
                async with self._HTTPClient__session.request(
                    method, url, **kwargs
                ) as r:
                    response_time = timer.stop()
                    # even errors have text involved in them so this is safe to call
                    data = await discord.http.json_or_text(r)

                    sanitized_url = sanitize_url(url)
                    without_ids = remove_ids(sanitized_url)

                    api_histogram.labels(method=method, path=without_ids).observe(
                        response_time
                    )

                    if (IGNORE_PATH not in url) or r.status not in [200, 204]:
                        http_logger.debug(
                            f"{method} {url} returned: {r.status}",
                            extra={
                                "http": {
                                    "method": method,
                                    "path": sanitized_url,
                                    "payload": kwargs.get("data"),
                                    "status": r.status,
                                    "response": data if 300 > r.status >= 200 else None,
                                }
                            },
                        )

                    # check if we have rate limit header information
                    remaining = r.headers.get("X-Ratelimit-Remaining")
                    if remaining == "0" and r.status != 429:
                        # we've depleted our current bucket
                        delta = discord.utils._parse_ratelimit_header(
                            r, use_clock=self.use_clock
                        )
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
                        if not r.headers.get("Via"):
                            # Banned by Cloudflare more than likely.
                            raise discord.errors.HTTPException(r, data)

                        # sleep a bit
                        retry_after = data["retry_after"] / 1000.0
                        http_logger.warning(
                            f"We are being rate limited. Retrying in {retry_after:.2f} "
                            f'seconds. Handled under the bucket "{bucket}"'
                        )

                        # check if it's a global rate limit
                        is_global = data.get("global", False)
                        if is_global:
                            http_logger.warning(
                                f"Global rate limit has been hit. "
                                f"Retrying in {retry_after:.2f} seconds."
                            )
                            self._global_over.clear()

                        await asyncio.sleep(retry_after)
                        http_logger.debug(
                            "Done sleeping for the rate limit. Retrying..."
                        )

                        # release the global lock now that the
                        # global rate limit has passed
                        if is_global:
                            self._global_over.set()
                            http_logger.debug("Global rate limit is now over.")

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
