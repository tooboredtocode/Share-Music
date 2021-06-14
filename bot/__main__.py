from discord_slash import SlashCommand

from bot.config import Tokens
from bot.factory import Bot


instance = Bot.create()
slash = SlashCommand(instance, override_type=True, sync_commands=True)
instance.load_extensions()
instance.run(Tokens.prod)
