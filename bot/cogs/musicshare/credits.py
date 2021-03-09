import time
from discord.ext import commands

from bot.bot import Bot


class Credits(commands.Cog):

    @commands.command()
    async def invite(self, ctx):
        ctx.send("Invite the bot here:\n"
                 "https://discord.com/api/oauth2/authorize?client_id=818620296410955808&permissions=0&scope=bot")

    @commands.command()
    async def source(self, ctx):
        ctx.send("You can find the code running this bot here:\n"
                 "https://github.com/tooboredtocode/Share-Music")

    @commands.command()
    async def credits(self, ctx):
        ctx.send("Built by albedo#9999\n"
                 "Powered by https://odesli.co")


def setup(bot: Bot):
    bot.add_cog(Credits(bot))
