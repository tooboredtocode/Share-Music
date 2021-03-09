import discord
import re
import requests

from discord.ext import commands

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
                     ".*youtube\.com)")

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

    @commands.command()
    async def share(self, ctx, url: str):
        try:
            await ctx.message.delete()
        except discord.errors.Forbidden:
            pass

        if not pattern.match(url):
            await ctx.send("Please send a valid url", delete_after=15)
            return

        message = await ctx.send("Please wait a moment...")

        result = requests.get(f"https://api.song.link/v1-alpha.1/links?url={url}")

        if result.status_code != 200:
            await ctx.send("Error getting links", delete_after=15)
            return

        result = result.json()

        links = list()

        for source in result["linksByPlatform"]:
            try:
                links.append(f"[{sources[source]}]({result['linksByPlatform'][source]['url']})")
            except KeyError:
                links.append(f"[{source}]({result['linksByPlatform'][source]['url']})")

        thumbnails = dict()

        for provider in result["entitiesByUniqueId"]:
            thumbnails[f"{result['entitiesByUniqueId'][provider]['apiProvider']}"]\
                = f"{result['entitiesByUniqueId'][provider]['thumbnailUrl']}"

        if "itunes" in thumbnails:
            thumbnail = thumbnails["itunes"]
        elif "spotify" in thumbnails:
            thumbnail = thumbnails["spotify"]
        else:
            thumbnail = result["entitiesByUniqueId"][result["entityUniqueId"]]["thumbnailUrl"]

        embed = discord.Embed.from_dict({
            "title": result["entitiesByUniqueId"][result["entityUniqueId"]]["title"],
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
                "name": result["entitiesByUniqueId"][result["entityUniqueId"]]["artistName"]
            },
            "fields": [{
                "name": "sent by",
                "value": ctx.author.mention
            }]
        })
         
        try:
            await message.edit(content="", embed=embed)
        except discord.errors.NotFound:
            await ctx.send(embed=embed)


def setup(bot: Bot):
    bot.add_cog(Share(bot))
