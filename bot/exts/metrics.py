import aiohttp
import discord
import json
import time

from discord.ext import commands, tasks
from functools import wraps
from loguru import logger
from typing import Union

from bot.config import Metrics as MetricsConf
from bot.factory import Bot
from bot.utils.metrics import api_gauge, api_processing_gauge, gateway_gauge, third_party_api_histogram, Timer


def time_it(func: callable) -> callable:
    @wraps(func)
    async def wrapper(*args, **kwargs):
        start = time.time()
        result = await func(*args, **kwargs)
        stop = time.time()

        duration = stop - start

        return (duration, result) if result is not None else duration

    return wrapper


class Metrics(commands.Cog):

    def __init__(self, bot: Bot):
        self.bot: Bot = bot
        self.session = aiohttp.ClientSession()
        self.channel: discord.channel = None
        self.metrics.start()

    def __del__(self):
        if not self.session.closed:
            if self.session.connector_owner:
                self.session.connector.close()
            self.session._connector = None

    def cog_unload(self):
        self.metrics.cancel()

    @time_it
    async def test_send(self):
        return await self.channel.send("Testing send")

    @time_it
    async def test_edit(self, message: discord.Message):
        await message.edit(content="Testing edit")

    @time_it
    async def test_delete(self, message: discord.Message):
        await message.delete()
        
    async def get_discord_stats(self) -> Union[int, None]:
        timer = Timer()
        async with self.session.get("https://discordstatus.com/metrics-display/5k2rt9f7pmny/day.json") as r:
            response_time = timer.stop()
            third_party_api_histogram.labels(
                method="GET",
                url="https://discordstatus.com/metrics-display"
            ).observe(response_time)
            
            if r.status != 200:
                body = ""
                try:
                    body = await r.text()
                except LookupError:
                    pass

                logger.warning(
                    f"Discord status returned: {r.status}",
                    extra={
                        "discord_status": {
                            "status": r.status,
                            "response": body
                        }
                    }
                )
                return None
            
            data = json.loads(await r.text(encoding='utf-8'))

            try:
                return data["summary"]["last"]
            except (KeyError, TypeError):
                return None

    @tasks.loop(seconds=MetricsConf.frequency)
    async def metrics(self):
        send_delay, message = await self.test_send()
        edit_delay = await self.test_edit(message)
        delete_delay = await self.test_delete(message)
        api_processing = await self.get_discord_stats()

        gateway_gauge.set(self.bot.latency)
        api_gauge.labels(action="send").set(send_delay)
        api_gauge.labels(action="edit").set(edit_delay)
        api_gauge.labels(action="delete").set(delete_delay)
        if api_processing is not None:
            api_processing_gauge.set(api_processing / 1000)

    @metrics.before_loop
    async def wait_for_bot(self):
        await self.bot.wait_until_ready()
        self.channel = await self.bot.fetch_channel(MetricsConf.channel)


def setup(bot: Bot):
    bot.add_cog(Metrics(bot))
