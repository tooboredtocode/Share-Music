import traceback
import discord

from discord.ext import commands
from discord_slash import SlashCommand

from config.config import token


class Bot(commands.Bot):
    def load_cogs(self, cogs: list):
        """Loads a list of cogs"""

        success, fail = 0, 0

        for cog in cogs:
            try:
                super().load_extension(cog)
                success += 1
            except Exception as e:
                print(f"Cog {cog} experienced an error during loading: {e}")
                fail += 1

        print(f"Cog loading complete! (Total: {success + fail} | Loaded: {success} | Failed: {fail})")

    async def on_error(self, event: str, *args, **kwargs):
        print(f"Runtime error: {event}\n{traceback.format_exc(limit=1750)}")
        traceback.print_exc()


def run(cogs: list, debug=False, prefix=None, help_command=None):
    if prefix is None:
        prefix = ["!"]
    bot = Bot(
        debug=debug,
        command_prefix=prefix,
        max_messages=10000,
        help_command=help_command,
        intents=discord.Intents.default()
    )

    slash = SlashCommand(bot, override_type=True, sync_commands=True)

    bot.load_cogs(cogs)
    bot.run(token)
