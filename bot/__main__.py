from bot.config import Tokens
from bot.factory import Bot, SlashCommand
from bot.utils import logging, monkey_patch, sentry

monkey_patch.patch()
sentry.configure()
logging.configure()

instance = Bot.create()
slash = SlashCommand(instance)
instance.load_extensions()
instance.run(Tokens.dev or Tokens.prod)
