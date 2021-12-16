from prometheus_client import start_http_server

from bot.config import Metrics as MetricsConf, General
from bot.factory import Bot, SlashCommand
from bot.utils import logging, monkey_patch, sentry

monkey_patch.patch()
sentry.configure()
logging.configure()

start_http_server(MetricsConf.port)
instance = Bot.create()
slash = SlashCommand(instance)
instance.load_extensions()
instance.run(General.token)
