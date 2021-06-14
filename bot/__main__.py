from discord_slash import SlashCommand

from bot.config import Tokens
from bot.factory import Bot
from bot.utils import logging

logging.configure()

instance = Bot.create()
slash = SlashCommand(instance, override_type=True, sync_commands=True)
instance.load_extensions()
instance.run(Tokens.prod)
