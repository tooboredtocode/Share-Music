import discord
import re
import requests

from discord.ext import commands
from discord_slash import cog_ext, SlashContext
from discord_slash.model import SlashCommandOptionType
from discord_slash.utils.manage_commands import create_option
from io import BytesIO
from random import randint
from PIL import Image

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

    @classmethod
    def get_dominant_colours(cls, url: str):

        # get the image from the url
        response = requests.get(url)
        thumbnail = Image.open(BytesIO(response.content))

        # downsize the image to increase processing and turn it into a palette
        thumbnail.thumbnail((150, 150))
        thumbnail = thumbnail.convert('P', palette=Image.WEB, colors=10)

        # get the most dominant colours
        palette = thumbnail.getpalette()
        color_counts = sorted(thumbnail.getcolors(), reverse=True)
        palette_index = color_counts[randint(0, 3)][1]
        dominant_color = palette[palette_index * 3:palette_index * 3 + 3]

        return tuple(dominant_color)

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
        for source, link in result["linksByPlatform"].items():
            title = source_identifier_to_name[source] if source in source_identifier_to_name else source
            url = link["url"]

            links.append(f"[{title}]({url})")

        # get important parts from the api response
        reduced_info = {}
        for key, value in result["entitiesByUniqueId"].items():
            provider = "original_provider" if key == result["entityUniqueId"] and value["apiProvider"] not in source_priority else value["apiProvider"]
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
        reduced_info = list(map(lambda key: reduced_info[key], sorted(reduced_info)))

        # get the information
        artist, title, thumbnail = [reduced_info[0][key] for key in ["artist", "title", "thumbnail"]]

        # get the dominant colours
        colour = self.get_dominant_colours(thumbnail)
        colour_int = (colour[0] << 16) + (colour[1] << 8) + colour[2]

        # create the discord embed
        embed = discord.Embed.from_dict({
            "title": title,
            "type": "rich",
            "color": colour_int,
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
