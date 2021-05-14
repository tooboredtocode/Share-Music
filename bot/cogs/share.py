import discord
import re
import requests

from discord.ext import commands
from discord_slash import cog_ext, SlashContext
from discord_slash.model import SlashCommandOptionType
from discord_slash.utils.manage_commands import create_option

from bot.bot import Bot

pattern = re.compile("^https:\/\/(?:"
                     ".*amazon\.com|"
                     ".*deezer\.com|"
                     ".*music\.apple\.com|"
                     ".*pandora.*\.com|"
                     "soundcloud\.com|"
                     ".*spotify\.com|"
                     ".*tidal\.com|"
                     ".*music\.yandex\..{1,3}|"
                     ".*youtu(?:\.be|be\.com))")

source_identifier_to_name = {
    "amazonMusic": "Amazon Music",
    "amazonStore": "Amazon",
    "deezer": "Deezer",
    "appleMusic": "Apple Music",
    "itunes": "iTunes",
    "pandora": "Pandora",
    "napster": "Napster",
    "soundcloud": "Soundcloud",
    "spotify": "Spotify",
    "tidal": "Tidal",
    "yandex": "Yandex",
    "youtube": "YouTube",
    "youtubeMusic": "YouTube Music"
}

source_priority = {
    "itunes": 1,
    "spotify": 2,
    "tidal": 3,
    "yandex": 4,
    "soundcloud": 5,
    "original_provider": 6
}


class Share(commands.Cog):

    @cog_ext.cog_slash(
        name="share",
        description="Share music to all platforms, using song.link's api",
        options=[
            create_option(
                name="url",
                description="The link for the song/album",
                option_type=SlashCommandOptionType.STRING,
                required=True
            )
        ]
    )
    async def _share(self, ctx: SlashContext, url: str):

        # filter out bad requests
        if not pattern.match(url):
            await ctx.send(hidden=True, content="Please send a valid url")
            return

        # send placeholder message
        await ctx.defer()

        # get the info from song.link
        response = requests.get(f"https://api.song.link/v1-alpha.1/links?url={url}")

        # inform user about error
        if response.status_code != 200:
            await ctx.send(content="Error getting links", delete_after=15)
            return

        # turn the request into a dict
        result = response.json()

        # get the links and store them with the markdown syntax already applied
        links = []
        for source in result["linksByPlatform"]:
            title = source_identifier_to_name[source] if source in source_identifier_to_name else source
            url = result["linksByPlatform"][source]["url"]

            links.append(f"[{title}]({url})")

        # get important parts from the api response
        reduced_info = {}
        for key, value in result["entitiesByUniqueId"].items():
            provider = value["apiProvider"] if key != result["entityUniqueId"] else "original_provider"
            if provider not in source_priority:
                continue
            try:
                reduced_info[source_priority[provider]] = {
                    "artist": value["artistName"],
                    "title": value["title"],
                    "thumbnail": value["thumbnailUrl"]
                }
            except KeyError:
                pass

        # sort the dict
        reduced_info = sorted(reduced_info).values()

        # get the information
        artist, title, thumbnail = [reduced_info[0][key] for key in ["artist", "title", "thumbnail"]]

        # create the discord embed
        embed = discord.Embed.from_dict({
            "title": title,
            "type": "rich",
            "description": f"{' | '.join(links)}",
            "url": f"{result['pageUrl']}",
            "footer": {
                "text": "Powered by odesli.co"
            },
            "thumbnail": {
                "url": thumbnail
            },
            "author": {
                "name": artist
            }
        })

        # send message
        await ctx.send(embed=embed)


def setup(bot: Bot):
    bot.add_cog(Share(bot))
