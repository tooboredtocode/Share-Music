from discord.ext import commands
from discord_slash import cog_ext, SlashContext
from prometheus_async.aio import time

from bot.factory import Bot
from bot.utils.metrics import command_histogram


class Credits(commands.Cog):

    def __init__(self, bot: Bot):
        self.bot = bot

    @cog_ext.cog_slash(
        name="invite",
        description="Get the invite link for the bot"
    )
    @time(command_histogram.labels(command="invite"))
    async def _invite(self, ctx: SlashContext):
        await ctx.send(hidden=True, content="Invite the bot here:\n"
                "https://share-music.albedo.me/")

    @cog_ext.cog_slash(
        name="source",
        description="Get the link to the bots source code"
    )
    @time(command_histogram.labels(command="source"))
    async def _source(self, ctx: SlashContext):
        await ctx.send(hidden=True, content="You can find the code running this bot here:\n"
                 "https://github.com/tooboredtocode/Share-Music")

    @cog_ext.cog_slash(
        name="credits",
        description="People and Projects making this work"
    )
    @time(command_histogram.labels(command="credits"))
    async def _credits(self, ctx: SlashContext):
        await ctx.send(hidden=True, content="Built by albedo#9999\n"
                 "Powered by https://odesli.co")


def setup(bot: Bot):
    bot.add_cog(Credits(bot))
