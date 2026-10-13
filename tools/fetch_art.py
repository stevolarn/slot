#!/usr/bin/env python3
"""Fetch box fronts, box backs, cart photos and game details for Shelf from
the LaunchBox Games Database, for the games on an SD card.

    python3 tools/fetch_art.py /media/$USER/SDCARD

Run it on a computer with the Brick's SD card in a reader. It reads the
card's Roms folder (one "Name (TAG)" folder per system, as NextUI lays them
out) and writes into Shelf's folder on the card, .userdata/tg5040/Shelf:

    Details/<TAG>/<game>.txt        year, genre, publisher, rating, players...
    Boxes/<TAG>/<game>.front.png    the box front, drawn large in Boxes and Store
    Boxes/<TAG>/<game>.back.png     the box back, which B turns to
    Carts/<TAG>/<game>.png          a photo of the real cart

where <game> is the ROM's file name without its extension. When the card has
Top 100 lists (Collections/Top 100, Roms/_Collections/Top 100 or
_Collections/Top 100), Shelf shows only their games, so only those are
fetched; --all fetches for every game in Roms.

The LaunchBox metadata (Metadata.zip, about 100 MB) is downloaded once and
kept in ~/.cache/shelf-art for a week. Pictures already on the card are
skipped, so a second run only fetches what is new; --force fetches them all
again. Pictures are shrunk to the size Shelf draws them (600 by 600 for
boxes, 360 by 360 for carts) and saved as PNG. The ROMs are only read.

Needs Python 3.8 or later and Pillow (pip install pillow).

Images and details come from the LaunchBox Games Database,
https://gamesdb.launchbox-app.com, contributed by its community.
"""
import argparse
import io
import os
import re
import sys
import time
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    sys.exit("fetch_art.py needs Pillow: pip install pillow")

METADATA_URL = "https://gamesdb.launchbox-app.com/Metadata.zip"
IMAGES_URL = "https://images.launchbox-app.com/"
USER_AGENT = "Shelf-fetch-art/1.0 (+https://github.com/stevolarn/slot)"
CREDIT = "Images and details: LaunchBox Games Database, https://gamesdb.launchbox-app.com"
CACHE = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "shelf-art"
METADATA_MAX_AGE = 7 * 24 * 3600
# Seconds between image downloads, to go easy on the image server.
PAUSE = 0.3

# Shelf's systems: the folder tags it shelves, as in shelf-store's scan.rs,
# and the LaunchBox platform each is filed under.
TAGS = {"GBA": "GBA", "MGBA": "GBA", "GB": "GB", "GBC": "GBC", "FC": "FC", "MD": "MD", "SFC": "SFC"}
PLATFORMS = {
    "GBA": "Nintendo Game Boy Advance",
    "GB": "Nintendo Game Boy",
    "GBC": "Nintendo Game Boy Color",
    "FC": "Nintendo Entertainment System",
    "MD": "Sega Genesis",
    "SFC": "Super Nintendo Entertainment System",
}
# A Game Boy Color game is often filed under Game Boy, and the other way round.
ALSO = {"GBC": ["GB"], "GB": ["GBC"]}
EXTENSIONS = {
    "GBA": {"gba"},
    "GB": {"gb", "gbc"},
    "GBC": {"gb", "gbc"},
    "FC": {"nes", "unf", "unif"},
    "MD": {"md", "gen", "smd", "bin"},
    "SFC": {"sfc", "smc", "fig", "swc", "bs"},
}
ARCHIVES = {"zip", "7z"}
REGIONS = ["Europe", "United Kingdom", "PAL", "World", "North America", "United States", None, "Japan"]
KINDS = ["fronts", "backs", "carts", "details"]
IMAGE_TYPES = {"fronts": ["Box - Front"], "backs": ["Box - Back"], "carts": ["Cart - Front", "Fanart - Cart - Front"]}
SIZES = {"fronts": 600, "backs": 600, "carts": 360}
FIELDS = ["Name", "ReleaseDate", "ReleaseYear", "Genres", "Developer", "Publisher", "MaxPlayers",
          "Cooperative", "CommunityRating", "CommunityRatingCount", "ESRB", "Overview"]
OVERVIEW_MAX = 700


def folder_tag(name):
    """The system a Roms folder is for: the tag in brackets at the end of its name."""
    if "satellaview" in name.lower():
        return None
    m = re.search(r"\(([^()]*)\)\s*$", name)
    return TAGS.get(m.group(1).upper()) if m else None


def norm(s):
    s = re.sub(r"\[[^\]]*\]|\([^)]*\)", "", s)
    s = re.sub(r"^(.*), (the|a|an)\b", r"\2 \1", s.strip(), flags=re.I)
    s = s.lower().replace("&", "and").replace("é", "e")
    s = re.sub(r"[^a-z0-9]+", " ", s)
    return re.sub(r"\s+", " ", s).strip()


def loose(s):
    """The name without a leading article, for a second try."""
    return re.sub(r"^(the|a|an) ", "", norm(s))


def squash(s):
    """The name with no spaces, so "Duck Tales" finds "DuckTales"."""
    return "~" + loose(s).replace(" ", "")


def one_line(s):
    return re.sub(r"\s+", " ", s or "").strip()


def clip(s, n):
    s = one_line(s)
    if len(s) <= n:
        return s
    cut = s[:n].rsplit(". ", 1)[0]
    return (cut if len(cut) > n // 2 else s[:n].rsplit(" ", 1)[0]) + "…"


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    return urllib.request.urlopen(req, timeout=120)


def metadata(given, refresh):
    """The path of Metadata.zip: the one given, or a cached download."""
    if given:
        return Path(given)
    path = CACHE / "Metadata.zip"
    fresh = path.is_file() and time.time() - path.stat().st_mtime < METADATA_MAX_AGE
    if fresh and not refresh:
        return path
    CACHE.mkdir(parents=True, exist_ok=True)
    print(f"downloading the LaunchBox metadata from {METADATA_URL} ...")
    tmp = path.with_suffix(".part")
    with fetch(METADATA_URL) as r, open(tmp, "wb") as f:
        while True:
            chunk = r.read(1 << 20)
            if not chunk:
                break
            f.write(chunk)
    os.replace(tmp, path)
    return path


def lists_on(card, roms):
    for d in (card / "Collections" / "Top 100", roms / "_Collections" / "Top 100", card / "_Collections" / "Top 100"):
        if d.is_dir():
            return d
    return None


def games_in_lists(lists, roms):
    """(tag, stem) of each game the Top 100 lists name, whose lines read
    /Roms/<system folder>/<rom>."""
    games = set()
    for lst in sorted(lists.glob("*.txt")):
        for line in lst.read_text(errors="replace").splitlines():
            line = line.strip()
            parts = Path(line).parts
            if len(parts) < 3:
                continue
            tag = folder_tag(parts[-2])
            if tag:
                games.add((tag, Path(line).stem))
    return games


def games_in_roms(roms):
    games = set()
    for system in sorted(roms.iterdir()):
        tag = folder_tag(system.name) if system.is_dir() and not system.name.startswith(".") else None
        if not tag:
            continue
        for rom in system.rglob("*"):
            if any(p.startswith(".") for p in rom.relative_to(system).parts) or not rom.is_file():
                continue
            ext = rom.suffix.lower().lstrip(".")
            if ext in EXTENSIONS[tag] or ext in ARCHIVES:
                games.add((tag, rom.stem))
    return games


def read_metadata(path, kinds):
    """The games of Shelf's systems in the metadata, the names that find them,
    and their pictures of the kinds wanted."""
    wanted_types = {t: k for k in kinds for t in IMAGE_TYPES.get(k, [])}
    platform_tag = {p: t for t, p in PLATFORMS.items()}
    names = {t: {} for t in PLATFORMS}
    games, tag_of, images = {}, {}, {}
    src = zipfile.ZipFile(path).open("Metadata.xml") if path.suffix == ".zip" else open(path, "rb")
    with src:
        for _, el in ET.iterparse(src):
            if el.tag == "Game":
                tag = platform_tag.get(el.findtext("Platform"))
                if tag:
                    dbid = el.findtext("DatabaseID")
                    games[dbid] = {f: el.findtext(f) for f in FIELDS}
                    tag_of[dbid] = tag
                    name = el.findtext("Name") or ""
                    for key in (norm(name), loose(name), squash(name)):
                        names[tag].setdefault(key, []).append(dbid)
                el.clear()
            elif el.tag == "GameAlternateName":
                dbid = el.findtext("DatabaseID")
                if dbid in tag_of:
                    alt = el.findtext("AlternateName") or ""
                    for key in (norm(alt), squash(alt)):
                        names[tag_of[dbid]].setdefault(key, []).append(dbid)
                el.clear()
            elif el.tag == "GameImage":
                kind = wanted_types.get(el.findtext("Type"))
                if kind:
                    fan = el.findtext("Type").startswith("Fanart")
                    images.setdefault((el.findtext("DatabaseID"), kind), []).append(
                        (fan, el.findtext("Region"), el.findtext("FileName")))
                el.clear()
            elif el.tag == "MameFile":
                el.clear()
    return names, games, images


def match(tag, stem, names, games):
    """The LaunchBox entry for a game: the one with the most votes among those
    its name finds, which is almost always the main release."""
    for t in [tag] + ALSO.get(tag, []):
        ids = names[t].get(norm(stem)) or names[t].get(loose(stem)) or names[t].get(squash(stem)) or []
        if not ids and " - " in stem:
            a, b = stem.split(" - ", 1)
            ids = names[t].get(norm(f"{a} {b}")) or []
        if ids:
            return max(ids, key=lambda i: int(games[i]["CommunityRatingCount"] or 0))
    return None


def details_text(g):
    year = (g["ReleaseDate"] or "")[:4] or (g["ReleaseYear"] or "")
    rows = {
        "name": one_line(g["Name"]),
        "year": year,
        "genre": one_line(g["Genres"]).replace(";", ","),
        "developer": one_line(g["Developer"]).replace(";", ","),
        "publisher": one_line(g["Publisher"]).replace(";", ","),
        "players": one_line(g["MaxPlayers"]),
        "coop": "yes" if (g["Cooperative"] or "").lower() == "true" else "",
        "rating": f"{float(g['CommunityRating']):.2f}" if g["CommunityRating"] else "",
        "votes": one_line(g["CommunityRatingCount"]),
        "esrb": one_line(g["ESRB"]).split(" - ")[0],
        "overview": clip(g["Overview"], OVERVIEW_MAX),
    }
    return "".join(f"{k}={v}\n" for k, v in rows.items() if v)


def best_image(found, regions):
    """A photo before fan art, then the first region in the order asked for."""
    def rank(r):
        fan, region, _ = r
        return (fan, regions.index(region) if region in regions else len(regions))
    return min(found, key=rank)


def save_picture(data, dest, kind):
    im = Image.open(io.BytesIO(data))
    if kind == "carts":
        im = im.convert("RGBA")
        box = im.getchannel("A").getbbox()
        if box:
            im = im.crop(box)
    else:
        im = im.convert("RGB")
    size = SIZES[kind]
    im.thumbnail((size, size))
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_name(dest.name + ".part")
    im.save(tmp, "PNG", optimize=True)
    os.replace(tmp, dest)


def dest_of(out, kind, tag, stem):
    return {
        "fronts": out / "Boxes" / tag / f"{stem}.front.png",
        "backs": out / "Boxes" / tag / f"{stem}.back.png",
        "carts": out / "Carts" / tag / f"{stem}.png",
        "details": out / "Details" / tag / f"{stem}.txt",
    }[kind]


def main():
    ap = argparse.ArgumentParser(
        description="Fetch box fronts, backs, cart photos and game details for Shelf from the LaunchBox Games Database.")
    ap.add_argument("card", nargs="?", help="where the SD card is mounted")
    ap.add_argument("--roms", help="the Roms folder (default: CARD/Roms)")
    ap.add_argument("--out", help="Shelf's folder (default: CARD/.userdata/tg5040/Shelf)")
    ap.add_argument("--lists", help="a folder of Top 100 lists to fetch for (default: the card's, if any)")
    ap.add_argument("--all", action="store_true", help="fetch for every game in Roms, not only the Top 100 lists")
    ap.add_argument("--only", default=",".join(KINDS),
                    help=f"what to fetch, comma separated (default: {','.join(KINDS)})")
    ap.add_argument("--regions", default=",".join(r or "none" for r in REGIONS),
                    help="the box regions to prefer, in order (default: Europe first)")
    ap.add_argument("--metadata", help="a Metadata.zip or Metadata.xml already downloaded")
    ap.add_argument("--refresh", action="store_true", help="download the metadata again even if a recent copy is kept")
    ap.add_argument("--force", action="store_true", help="fetch pictures and details already on the card again")
    args = ap.parse_args()

    card = Path(args.card) if args.card else None
    if not card and not (args.roms and args.out):
        ap.error("give the SD card's folder, or both --roms and --out")
    roms = Path(args.roms) if args.roms else card / "Roms"
    out = Path(args.out) if args.out else card / ".userdata" / "tg5040" / "Shelf"
    if not roms.is_dir():
        sys.exit(f"no Roms folder at {roms}")
    kinds = [k.strip() for k in args.only.split(",") if k.strip()]
    if bad := [k for k in kinds if k not in KINDS]:
        ap.error(f"unknown kind {', '.join(bad)}; choose from {', '.join(KINDS)}")
    regions = [None if r.lower() == "none" else r for r in args.regions.split(",")]

    lists = Path(args.lists) if args.lists else (None if args.all or not card else lists_on(card, roms))
    if lists and not args.all:
        games = games_in_lists(lists, roms)
        print(f"{len(games)} games in the lists at {lists}")
    else:
        games = games_in_roms(roms)
        print(f"{len(games)} games in {roms}")
    if not games:
        return

    meta = metadata(args.metadata, args.refresh)
    print(f"reading {meta} ...")
    names, entries, images = read_metadata(meta, kinds)

    counts = {k: [0, 0, 0] for k in kinds}  # fetched, already there, none found
    unmatched = []
    stopped = False
    for tag, stem in sorted(games):
        dbid = match(tag, stem, names, entries)
        if not dbid:
            unmatched.append(f"{tag}/{stem}")
            continue
        for kind in kinds:
            dest = dest_of(out, kind, tag, stem)
            if kind == "details":
                text = details_text(entries[dbid])
                if dest.is_file() and dest.read_text() == text:
                    counts[kind][1] += 1
                    continue
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_text(text)
                counts[kind][0] += 1
                continue
            if dest.is_file() and not args.force:
                counts[kind][1] += 1
                continue
            if stopped:
                continue
            found = images.get((dbid, kind))
            if not found:
                counts[kind][2] += 1
                continue
            url = IMAGES_URL + best_image(found, regions)[2]
            try:
                with fetch(url) as r:
                    data = r.read()
                save_picture(data, dest, kind)
                counts[kind][0] += 1
            except urllib.error.HTTPError as e:
                if e.code == 429:
                    print("the image server asked us to slow down; stopping. Run again later to fetch the rest.")
                    stopped = True
                    continue
                print(f"  {tag}/{stem}: {kind}: HTTP {e.code}")
                counts[kind][2] += 1
            except (urllib.error.URLError, OSError) as e:
                print(f"  {tag}/{stem}: {kind}: {e}")
                counts[kind][2] += 1
            time.sleep(PAUSE)
        done = sum(c[0] for c in counts.values())
        if done and done % 50 == 0:
            print(f"  {done} fetched so far")

    for kind in kinds:
        new, had, none = counts[kind]
        print(f"{kind}: {new} fetched, {had} already there, {none} not found")
    if unmatched:
        print(f"{len(unmatched)} games not found in LaunchBox:")
        for g in unmatched:
            print(f"  {g}")
    print(f"written to {out}")
    print(CREDIT)


if __name__ == "__main__":
    main()
