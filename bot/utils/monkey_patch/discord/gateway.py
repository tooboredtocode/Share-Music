import discord
import json

from loguru import logger

from bot.config import Metrics as MetricsConf
from bot.utils.monkey_patch import remove_sensitive_info

op_codes = {
    0: "",  # ignore this code so that the event can be properly displayed
    1: "Heartbeat",
    2: "Identify",
    3: "Presence Update",
    4: "Voice State Update",
    6: "Resume",
    7: "Reconnect",
    8: "Request Guild Members",
    9: "Invalid Session",
    10: "Hello",
    11: "Heartbeat ACK",
}

gateway_logger = logger.patch(lambda record: record.update(name="discord.gateway"))


def log_gateway_events(data):
    remove_sensitive_info(data, ["token"])

    direction = data["gateway"].get("direction")
    event = data["gateway"].get("t")
    op_code = data["gateway"].get("op")

    gateway_logger.debug(
        f"{'Received' if direction == 'in' else 'Dispatched'} gateway event "
        f"{op_codes[op_code]}{event if event else ''}",
        extra=data
    )


async def send_as_json(self, data):
    await discord.gateway.DiscordWebSocket.send_heartbeat_copy(self, data)
    data["direction"] = "out"
    log_gateway_events({"gateway": data})


async def send_heartbeat(self, data):
    await discord.gateway.DiscordWebSocket.send_as_json_copy(self, data)
    data["direction"] = "out"
    log_gateway_events({"gateway": data})


IGNORED_CHANNEL_STRING = str(MetricsConf.channel)


async def received_message(self, msg):
    if type(msg) is bytes:
        self._buffer.extend(msg)

        if len(msg) < 4 or msg[-4:] != b'\x00\x00\xff\xff':
            return
        msg = self._zlib.decompress(self._buffer)
        msg = msg.decode('utf-8')
        self._buffer = bytearray()
    data = json.loads(msg)
    data["direction"] = "in"
    channel_id = 0

    try:
        channel_id = data["d"]["channel_id"]
    except (KeyError, TypeError):
        pass

    if channel_id != IGNORED_CHANNEL_STRING:
        log_gateway_events({"gateway": data})

    await discord.gateway.DiscordWebSocket.received_message_copy(self, msg)
