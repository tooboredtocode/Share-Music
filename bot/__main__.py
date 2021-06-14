from discord_slash import SlashCommand

from config.config import token
from bot.factory import Bot


instance = Bot.create()
slash = SlashCommand(instance, override_type=True, sync_commands=True)
instance.load_extensions()
instance.run(token)

