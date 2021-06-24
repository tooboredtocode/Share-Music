import discord
import discord_slash

from discord.ext import commands
from loguru import logger
from sentry_sdk import capture_exception

from bot.config import General


class Bot(commands.Bot):

    @classmethod
    def create(cls) -> "Bot":

        intents = discord.Intents.all()
        intents.presences = False
        intents.members = False

        return cls(
            command_prefix=General.prefix,
            max_messages=10_000,
            allowed_mentions=discord.AllowedMentions(everyone=False, roles=False, users=False),
            intents=intents,
        )

    def load_extensions(self):
        from bot.utils.extensions import EXTENSIONS

        extensions = set(EXTENSIONS)
        extensions.add("jishaku")

        success, fail = 0, 0
        for extension in extensions:
            try:
                self.load_extension(extension)
                success += 1
            except Exception as e:
                logger.error(f"Cog {extension} experienced an error during loading: {e}")
                fail += 1

        logger.info(f"Cog loading complete! (Total: {success + fail} | Loaded: {success} | Failed: {fail})")

    async def on_error(self, event: str, *args, **kwargs):
        capture_exception()
        logger.exception(f"Runtime error in {event}\n")

    async def on_command_error(self, context, exception):
        if self.extra_events.get("on_command_error", None):
            return

        if hasattr(context.command, "on_error"):
            return

        cog = context.cog
        if cog and commands.Cog._get_overridden_method(cog.cog_command_error) is not None:
            return
        
        if isinstance(exception, (
            commands.UserInputError,
            commands.CheckFailure,
            commands.CommandNotFound,
            commands.CommandOnCooldown,
            commands.ArgumentParsingError
        )):
            logger.opt(exception=True).info(f"Ignoring exception in command {context.command}:\n{exception}\n")
        else:
            capture_exception(exception)
            logger.exception(f"Ignoring exception in command {context.command}:\n{exception}\n")


class SlashCommand(discord_slash.SlashCommand):

    @classmethod
    def create(cls, client: Bot):
        return cls(
            client=client,
            sync_commands=True,
            delete_from_unused_guilds=True,
            sync_on_cog_reload=True,
            override_type=True
        )
