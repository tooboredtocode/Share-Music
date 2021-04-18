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

sources = {
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

        # get the links
        links = list()
        for source in result["linksByPlatform"]:
            try:
                links.append(f"[{sources[source]}]({result['linksByPlatform'][source]['url']})")
            except KeyError:
                links.append(f"[{source}]({result['linksByPlatform'][source]['url']})")

        # get the song/album info
        song_album_info = dict()
        for provider in result["entitiesByUniqueId"]:
            try:
                song_album_info[f"{result['entitiesByUniqueId'][provider]['apiProvider']}"] = {
                    "artist": result["entitiesByUniqueId"][provider]["artistName"],
                    "title": result["entitiesByUniqueId"][provider]["title"],
                    "thumbnail": result["entitiesByUniqueId"][provider]["thumbnailUrl"]
                }
            except KeyError:
                pass

        # set the artist, title and thumbnail variables, since the data provided by youtube isn't very good
        if "itunes" in song_album_info:
            artist = song_album_info["itunes"]["artist"]
            title = song_album_info["itunes"]["title"]
            thumbnail = song_album_info["itunes"]["thumbnail"]
        elif "spotify" in song_album_info:
            artist = song_album_info["itunes"]["artist"]
            title = song_album_info["itunes"]["title"]
            thumbnail = song_album_info["itunes"]["thumbnail"]
        elif "tidal" in song_album_info:
            artist = song_album_info["tidal"]["artist"]
            title = song_album_info["tidal"]["title"]
            thumbnail = song_album_info["tidal"]["thumbnail"]
        elif "yandex" in song_album_info:
            artist = song_album_info["yandex"]["artist"]
            title = song_album_info["yandex"]["title"]
            thumbnail = song_album_info["yandex"]["thumbnail"]
        elif "soundcloud" in song_album_info:
            artist = song_album_info["soundcloud"]["artist"]
            title = song_album_info["soundcloud"]["title"]
            thumbnail = song_album_info["soundcloud"]["thumbnail"]
        else:
            artist = result["entitiesByUniqueId"][result["entityUniqueId"]]["artistName"]
            title = result["entitiesByUniqueId"][result["entityUniqueId"]]["title"]
            thumbnail = result["entitiesByUniqueId"][result["entityUniqueId"]]["thumbnailUrl"]

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
