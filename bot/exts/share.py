import discord
import re
import aiohttp

from discord.ext import commands
from discord_slash import cog_ext, SlashContext
from discord_slash.model import SlashCommandOptionType
from discord_slash.utils.manage_commands import create_option
from io import BytesIO
from json import JSONDecodeError
from loguru import logger
from prometheus_async.aio import time
from random import randint
from sentry_sdk import capture_exception
from PIL import Image

from bot.factory import Bot
from bot.utils.metrics import command_histogram, third_party_api_histogram, Timer

PATTERN = re.compile(
    "^https:\/\/(?:"
    ".*amazon\.com|"
    ".*deezer\.com|"
    ".*music\.apple\.com|"
    ".*pandora.*\.com|"
    "soundcloud\.com|"
    ".*spotify\.com|"
    ".*tidal\.com|"
    ".*music\.yandex\..{1,3}|"
    ".*youtu(?:\.be|be\.com))"
)

SOURCE_IDENTIFIER_TO_NAME = {
    "spotify": "Spotify",
    "itunes": "iTunes",
    "appleMusic": "Apple Music",
    "youtube": "YouTube",
    "youtubeMusic": "YouTube Music",
    "googleStore": "Google Store",
    "pandora": "Pandora",
    "deezer": "Deezer",
    "tidal": "Tidal",
    "amazonStore": "Amazon Store",
    "amazonMusic": "Amazon Music",
    "soundcloud": "SoundCloud",
    "napster": "Napster",
    "yandex": "Yandex",
    "spinrilla": "Spinrilla",
    "audius": "Audius",
}

SUPPORTED_SOURCES = [
    "Spotify",
    "iTunes",
    "Apple Music",
    "YouTube",
    "YouTube Music",
    "Pandora",
    "Deezer",
    "Tidal",
    "Amazon Music",
    "SoundCloud",
    "Yandex",
]

SOURCE_PRIORITY = ["itunes", "spotify", "tidal", "yandex", "soundcloud"]


def remove_unique_ids(url: str) -> str:
    path_components = url.split("/")

    return "/".join(path_components[:4])


class Share(commands.Cog):
    def __init__(self, bot: Bot):
        self.bot = bot
        self.session = aiohttp.ClientSession()

    def __del__(self):
        if not self.session.closed:
            if self.session.connector_owner:
                self.session.connector.close()
            self.session._connector = None

    async def get_dominant_colours(self, url: str) -> tuple[int, int, int]:

        if not url:
            return 0, 0, 0

        # get the image from the url
        timer = Timer()
        async with self.session.get(url) as response:
            response_time = timer.stop()
            third_party_api_histogram.labels(
                method="GET", url=remove_unique_ids(url)
            ).observe(response_time)

            logger.debug(
                f"GET {url} returned: {response.status}",
                extra={
                    "share_music": {
                        "method": "GET",
                        "path": url,
                        "status": response.status,
                    }
                },
            )

            if response.status != 200:
                return 0, 0, 0
            thumbnail = Image.open(BytesIO(await response.read()))

            # downsize the image to increase processing and turn it into a palette
            thumbnail.thumbnail((150, 150))
            thumbnail = thumbnail.convert("P", palette=Image.WEB, colors=10)

            # get the most dominant colours
            palette = thumbnail.getpalette()
            color_counts = sorted(thumbnail.getcolors(), reverse=True)
            palette_index = color_counts[randint(0, 3)][1]
            dominant_color = palette[palette_index * 3 : palette_index * 3 + 3]

            return tuple(dominant_color)

    @cog_ext.cog_slash(
        name="share",
        description="Share music to all platforms, using song.link's api",
        options=[
            create_option(
                name="url",
                description="The link for the song/album",
                option_type=SlashCommandOptionType.STRING,
                required=True,
            )
        ],
    )
    @time(command_histogram.labels(command="share"))
    async def _share(self, ctx: SlashContext, url: str):

        # filter out bad requests
        if not PATTERN.match(url):
            await ctx.send(
                hidden=True,
                content=f"Please send a valid url, I can only work with links from the following platforms:\n"
                f"({', '.join(SUPPORTED_SOURCES[:-1])} and {SUPPORTED_SOURCES[-1]})",
            )
            return

        # send placeholder message
        await ctx.defer()

        timer = Timer()
        # get the info from song.link
        async with self.session.get(
            f"https://api.song.link/v1-alpha.1/links?url={url}"
        ) as response:
            response_time = timer.stop()
            third_party_api_histogram.labels(
                method="GET", url="https://api.song.link/v1-alpha.1/links"
            ).observe(response_time)

            body = await response.text()

            logger.debug(
                f"GET {url} returned: {response.status}",
                extra={
                    "share_music": {
                        "method": "GET",
                        "path": f"https://api.song.link/v1-alpha.1/links?url={url}",
                        "status": response.status,
                        "response": "expected" if response.status == 200 else body,
                    }
                },
            )

            # inform user about error
            if response.status != 200:
                await ctx.send(
                    content="Error getting links, song.link couldn't respond",
                    delete_after=15,
                )
                logger.info(
                    f"Couldn't get response from song.link for url: {url} "
                    f"song.link responded with code: {response.status}"
                )
                return

            # turn the request into a dict
            try:
                result = await response.json()
            except JSONDecodeError as e:
                await ctx.send(
                    content="Error getting links, song.link returned an unexpected response",
                    delete_after=15,
                )
                logger.opt(exception=True).warning(
                    f"song.link returned a faulty response for url: {url}, error: {e}"
                )
                return

            # wrap the whole thing in a try loop if certain keys aren't found
            try:
                # get the links and store them with the markdown syntax already applied
                links = []
                for source, link in result["linksByPlatform"].items():
                    title = SOURCE_IDENTIFIER_TO_NAME.get(source) or source
                    url = link["url"]

                    links.append(f"[{title}]({url})")

                links.sort(key=lambda chars: chars.upper())

                # Fix some nasty references later
                artist = None
                title = None
                thumbnail = None
                colour_int = None

                # get important parts from the api response
                sp = SOURCE_PRIORITY.copy()
                sp.append(
                    result["entitiesByUniqueId"][result["entityUniqueId"]][
                        "apiProvider"
                    ]
                )
                current_max = len(sp) - 1
                for key, value in result["entitiesByUniqueId"].items():
                    if not ((provider := value.get("apiProvider")) in sp):
                        continue

                    if (priority := sp.index(provider)) > current_max:
                        continue
                    current_max = priority

                    try:
                        artist = value["artistName"]
                        title = value["title"]
                        thumbnail = value["thumbnailUrl"]
                    except KeyError:
                        pass

            except KeyError as e:
                await ctx.send(
                    content="Error getting links, song.link returned unexpected response",
                    delete_after=15,
                )
                capture_exception(e)
                logger.opt(exception=True).warning(
                    f"song.link returned a faulty response for url: {url}, KeyError: {e}"
                )
                return

            # get the dominant colours
            colour = await self.get_dominant_colours(thumbnail)
            colour_int = (colour[0] << 16) + (colour[1] << 8) + colour[2]

            # create the discord embed
            embed = discord.Embed.from_dict(
                {
                    "title": title,
                    "type": "rich",
                    "color": colour_int,
                    "description": f"{' | '.join(links)}",
                    "url": f"{result['pageUrl']}",
                    "footer": {"text": "Powered by odesli.co"},
                    "thumbnail": {"url": thumbnail},
                    "author": {"name": artist},
                }
            )

            # send message
            await ctx.send(embed=embed)


def setup(bot: Bot):
    bot.add_cog(Share(bot))
