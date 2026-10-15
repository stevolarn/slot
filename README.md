# Slot

A NextUI pak for the TrimUI Brick that turns your game library into a shelf of
real-looking carts and boxes. You browse the shelf, put a cart in the slot, and
Slot hands the game to NextUI's own emulator. When the game quits you are back
at the shelf.

Slot does not emulate anything itself, so it plays whatever your NextUI card
already plays. Games must be unzipped where NextUI needs them to be.

Release notes are in the [changelog](CHANGELOG.md).

---

## Features

- One shelf per console, plus Recently Played and your own Favourites (Recently Rented and Favourite Rentals in the video store).
- Staff Picks (Picked for You): games you left part way, and good ones you have never played, chosen afresh each day.
- Shows only the games in your Top 100 collection lists when the card has them.
- Real cart photos and labels, drawn for each console's own style of cart.
- Two themes, picked in Menu, Options, Theme, each shown as boxes or carts (Options, View, or L2 to swap):
  - **Original**: a row of carts over the console's slot, or each box front large in the dark, turned over or opened for its cart. Plain words: Favourites, Recently Played, MOST PLAYED.
  - **Video store**: a 90s rental shop, with one big box (or a loose cart) on the shelf under a hanging aisle sign, a RENTED tag counting your plays, and the console's name on a ticket beside a VCR clock.
- Box fronts and backs, using the box art NextUI already keeps in `.media`.
- Blow on a cart and it tips up to show its contacts, with a breath over them. Carts left unplayed gather dust.
- A CONTINUE? note on games with a save.
- A random spin to pick a game for you.
- Search (Menu, Search): spell a name arcade-style and jump straight to the game.
- Hide a game you never want to see (Menu, Hide Game); Options, Show Hidden brings them back to unhide.
- Each console's shelf opens where you left it, even after playing a game from another.
- Sort each shelf by name, genre, year, publisher, rating or plays (R2), from the details LaunchBox gave each game. The aisle sign names the section, such as TOP RENTALS or NEW RELEASES.
- A CRT mode, with scanlines and curved corners.
- Art loads in the background, so browsing never waits on the SD card.

## Artwork and details

On the Brick, **Menu > Get Artwork** fetches box fronts, box backs, cart photos and game details from the [LaunchBox Games Database](https://gamesdb.launchbox-app.com) over Wi-Fi, for the games on your shelves. It shows how much room the pictures need before it starts, and B stops it. LaunchBox's list of games is read as it downloads and never saved: only a small index of your games is kept, in `Config/launchbox.txt`. Box scans are saved as JPEG at the size Slot draws them (about 90 KB each), so a few hundred games take a few hundred MB at most. Pictures already on the card are skipped, so running it again only fetches what is new. Restart Slot afterwards to see them.

From a computer, `tools/fetch_art.py` fills a card with box fronts, box backs, cart photos and game details from the [LaunchBox Games Database](https://gamesdb.launchbox-app.com), for the games on it (only the Top 100 games when the card has those lists). Put the SD card in a computer and run:

```
pip install pillow
python3 tools/fetch_art.py /media/$USER/SDCARD
```

Pictures already on the card are skipped, so running it again only fetches what is new. See `python3 tools/fetch_art.py --help` for the options.

## Buttons

| Button     | On the shelf                                         |
|------------|------------------------------------------------------|
| A          | Insert the cart and play. In the store and on the box shelf, A takes the cart out over the console and A again plays it. |
| B          | Turn the box over, or turn a cart round to its box (put a cart back once it is out) |
| D-pad      | Browse                                               |
| Up / Down  | Change system                                        |
| L / R      | Jump by letter, or by aisle when sorted              |
| X          | Blow on the cart; on the back of a saved game's box, play it from the start |
| Y          | Add to or remove from Favourites                     |
| Select     | Random game                                          |
| L2         | Swap boxes and carts, in the theme picked in Options |
| R2         | Change the sort order                                |
| Start      | Button guide                                         |
| Menu       | Menu and settings                                    |

Favourite Rentals are kept in `Config/favourites.txt` under `.userdata/tg5040/Slot`, newest first.

---

## Install

Slot needs NextUI on a TrimUI Brick.

1. Download `slot-brick-pak.zip` from the
   [releases](https://github.com/stevolarn/slot/releases).
2. Unzip it and copy the `Tools` folder onto the root of the Brick's SD card.
3. Put the card in the Brick and open Tools, then Slot.

Slot keeps its settings and art in `.userdata/tg5040/Slot` on the card.

Or use `card/tg5040/install-sd.sh`, which copies the pak and your art to a card
in a reader, or straight onto a Brick over the network:

```sh
card/tg5040/install-sd.sh /media/$USER/SDCARD slot-brick-pak.zip
card/tg5040/install-sd.sh ftp://192.168.1.231/ slot-brick-pak.zip
```

The top of that script lists what it can do and the `SHELF_*` settings it takes.

### Coming from slot

Slot used to be called Shelf (and before that slot). Starting it once renames
`.userdata/tg5040/Slot` to `.userdata/tg5040/Slot`, so the art and settings carry
over. The old `Tools/tg5040/Slot.pak` is not removed for you: delete it so only
Slot shows in Tools, or run the installer on a card in a reader with
`SHELF_REMOVE_OLD=1`.

## Art

Slot draws plain carts out of the box. For the real thing, put pictures in
`.userdata/tg5040/Slot`, one folder per console, named after the game's rom:

| Folder                                | What goes in it                      |
|---------------------------------------|--------------------------------------|
| `Carts/<console>/<rom name>.png`      | A photo of the real cart             |
| `Labels/<console>/<rom name>.png`     | A label for the console's cart       |
| `Boxes/<console>/<rom name>.back.png` | A scan of the back of the box        |

A game with no picture of its own takes NextUI's box art from the `.media` folder
beside its rom. Slot shrinks and caches what it reads, so only the first
start after new art is added is slower. `card/tg5040/install-sd.sh` shrinks
pictures to the size Slot draws them before it sends them to the card.

---

## Building

Slot is Rust, in a cargo workspace under `crates/`. Tests and a window preview
run on a laptop:

```sh
cargo test --workspace
SHELF_ROOT=./sdcard cargo run --release -p shelf
```

The Brick pak is a 64-bit Arm build for glibc 2.31. With
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) and zig:

```sh
cargo zigbuild --profile device --target aarch64-unknown-linux-gnu.2.31 \
  --target-dir target-device -p shelf --no-default-features --features device
```

or `task dist:brick`, which builds in an arm64 container and assembles
`dist-brick/Tools/tg5040/Slot.pak` and `dist-brick/slot-brick-pak.zip`. The
`card/test-slotpak.sh` script tests the pak's launch script off the device.

---

## Credits and licence

Slot is free software under the GPL-3.0-or-later; see [LICENSE](LICENSE).

It is based on [slot](https://github.com/BrandonKowalski/slot) by Brandon T.
Kowalski, copyright 2026 Brandon T. Kowalski, which began as a Game Boy frontend
for Anbernic handhelds. The cart and shelf drawing, the cart sounds and the
sticker owe to that work. Type is Open Sans and Nerd Fonts Symbols by Ryan L
McIntyre.

The Rust code was put together with Claude and reviewed by hand.
