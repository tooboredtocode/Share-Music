from discord.ext import commands
from discord_slash import cog_ext, SlashContext

from bot.bot import Bot


class Credits(commands.Cog):

    @cog_ext.cog_slash(
        name="invite",
        description="Get the invite link from the bot"
    )
    async def _invite(self, ctx: SlashContext):
        await ctx.send(hidden=True, content="Invite the bot here:\n"
                "https://share-music.albedo.me/")

    @cog_ext.cog_slash(
        name="source",
        description="Get the link to the bots source code"
    )
    async def _source(self, ctx: SlashContext):
        await ctx.send(hidden=True, content="You can find the code running this bot here:\n"
                 "https://github.com/tooboredtocode/Share-Music")

    @cog_ext.cog_slash(
        name="credits"
    )
    async def _credits(self, ctx: SlashContext):
        await ctx.send(hidden=True, content="Built by albedo#9999\n"
                 "Powered by https://odesli.co")


def setup(bot: Bot):
    bot.add_cog(Credits(bot))
