def remove_sensitive_info(data, blacklist):
    for key, value in data.items():
        if isinstance(value, dict):
            remove_sensitive_info(value, blacklist)
        for test in blacklist:
            if key == test:
                data[key] = "redacted"


def patch():
    import discord

    from bot.utils.monkey_patch.discord import gateway, http

    discord.gateway.DiscordWebSocket.send_as_json_copy = (
        discord.gateway.DiscordWebSocket.send_as_json
    )
    discord.gateway.DiscordWebSocket.send_as_json = gateway.send_as_json

    discord.gateway.DiscordWebSocket.send_heartbeat_copy = (
        discord.gateway.DiscordWebSocket.send_heartbeat
    )
    discord.gateway.DiscordWebSocket.send_heartbeat = gateway.send_heartbeat

    discord.gateway.DiscordWebSocket.received_message_copy = (
        discord.gateway.DiscordWebSocket.received_message
    )
    discord.gateway.DiscordWebSocket.received_message = gateway.received_message

    discord.http.HTTPClient.request = http.request
