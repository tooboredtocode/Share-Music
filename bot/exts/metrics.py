import discord
import time

from discord.ext import commands, tasks
from functools import wraps

from bot.config import Metrics as MetricsConf
from bot.factory import Bot
from bot.utils.metrics import gateway_gauge, api_gauge


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
        self.channel: discord.channel = None
        self.metrics.start()

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

    @tasks.loop(seconds=MetricsConf.frequency)
    async def metrics(self):
        send_delay, message = await self.test_send()
        edit_delay = await self.test_edit(message)
        delete_delay = await self.test_delete(message)

        gateway_gauge.set(self.bot.latency)
        api_gauge.labels(action="send").set(send_delay)
        api_gauge.labels(action="edit").set(edit_delay)
        api_gauge.labels(action="delete").set(delete_delay)

    @metrics.before_loop
    async def wait_for_bot(self):
        await self.bot.wait_until_ready()
        self.channel = await self.bot.fetch_channel(MetricsConf.channel)


def setup(bot: Bot):
    bot.add_cog(Metrics(bot))
