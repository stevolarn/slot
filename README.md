# Shelf

A NextUI pak for the TrimUI Brick that turns your game library into a shelf of
real-looking carts and boxes. You browse the shelf, put a cart in the slot, and
Shelf hands the game to NextUI's own emulator. When the game quits you are back
at the shelf.

Shelf does not emulate anything itself, so it plays whatever your NextUI card
already plays. Games must be unzipped where NextUI needs them to be.

Release notes are in the [changelog](CHANGELOG.md).

---

## Features

- One shelf per console, plus Recently Played and your own Favourites (Recently Rented and Favourite Rentals in the video store).
- Shows only the games in your Top 100 collection lists when the card has them.
- Real cart photos and labels, drawn for each console's own style of cart.
- Two themes, picked in Menu, Options, Theme, each shown as boxes or carts (Options, View, or L2 to swap):
  - **Original**: a row of carts over the console's slot, or each box front large in the dark, turned over or opened for its cart. Plain words: Favourites, Recently Played, MOST PLAYED.
  - **Video store**: a 90s rental shop, with one big box (or a loose cart) on the shelf under a hanging aisle sign, a RENTED tag counting your plays, and the console's name on a ticket beside a VCR clock.
- Box fronts and backs, using the box art NextUI already keeps in `.media`.
- Blow on a cart and it tips up to show its contacts, with a breath over them. Carts left unplayed gather dust.
- A CONTINUE? note on games with a save.
- A random spin to pick a game for you.
- Sort each shelf by name, genre, year, publisher, rating or plays (R2), from the details LaunchBox gave each game. The aisle sign names the section, such as TOP RENTALS or NEW RELEASES.
- A CRT mode, with scanlines and curved corners.
- Art loads in the background, so browsing never waits on the SD card.

## Buttons

| Button     | On the shelf                                         |
|------------|------------------------------------------------------|
| A          | Insert the cart and play. In the store and on the box shelf, A takes the cart out over the console and A again plays it. |
| B          | Turn the box over, or turn a cart round to its box (put a cart back once it is out) |
| D-pad      | Browse                                               |
| Up / Down  | Change system                                        |
| L / R      | Jump by letter, or by aisle when sorted              |
| X          | Blow on the cart                                     |
| Y          | Add to or remove from Favourites                     |
| Select     | Random game                                          |
| L2         | Swap boxes and carts, in the theme picked in Options |
| R2         | Change the sort order                                |
| Start      | Button guide                                         |
| Menu       | Menu and settings                                    |

Favourite Rentals are kept in `Config/favourites.txt` under `.userdata/tg5040/Shelf`, newest first.

------------|-----------------------------|
| A          | Insert the cart and play    |
| D-pad      | Browse                      |
| Up / Down  | Change system               |
| L / R      | Jump by letter, or by aisle when sorted |
| X          | Blow on the cart            |
| B          | Box front / back            |
| Select     | Random game                 |
| L2         | Boxes or carts              |
| R2         | Change the sort order       |
| Start      | Power menu                  |
| Menu       | Settings                    |
| Y          | Button guide                |

---

## Install

Shelf needs NextUI on a TrimUI Brick.

1. Download `shelf-brick-pak.zip` from the
   [releases](https://github.com/stevolarn/slot/releases).
2. Unzip it and copy the `Tools` folder onto the root of the Brick's SD card.
3. Put the card in the Brick and open Tools, then Shelf.

Shelf keeps its settings and art in `.userdata/tg5040/Shelf` on the card.

Or use `card/tg5040/install-sd.sh`, which copies the pak and your art to a card
in a reader, or straight onto a Brick over the network:

```sh
card/tg5040/install-sd.sh /media/$USER/SDCARD shelf-brick-pak.zip
card/tg5040/install-sd.sh ftp://192.168.1.231/ shelf-brick-pak.zip
```

The top of that script lists what it can do and the `SHELF_*` settings it takes.

### Coming from slot

Shelf used to be called slot. Starting it once renames `.userdata/tg5040/slot` to
`.userdata/tg5040/Shelf`, so the art and settings carry over. The old
`Tools/tg5040/slot.pak` is not removed for you: delete it so only Shelf shows in
Tools. The installer prints a reminder, and deletes it when run with
`SHELF_REMOVE_OLD=1`.

## Art

Shelf draws plain carts out of the box. For the real thing, put pictures in
`.userdata/tg5040/Shelf`, one folder per console, named after the game's rom:

| Folder                                | What goes in it                      |
|---------------------------------------|--------------------------------------|
| `Carts/<console>/<rom name>.png`      | A photo of the real cart             |
| `Labels/<console>/<rom name>.png`     | A label for the console's cart       |
| `Boxes/<console>/<rom name>.back.png` | A scan of the back of the box        |

A game with no picture of its own takes NextUI's box art from the `.media` folder
beside its rom. Shelf shrinks and caches what it reads, so only the first
start after new art is added is slower. `card/tg5040/install-sd.sh` shrinks
pictures to the size Shelf draws them before it sends them to the card.

---

## Building

Shelf is Rust, in a cargo workspace under `crates/`. Tests and a window preview
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
`dist-brick/Tools/tg5040/Shelf.pak` and `dist-brick/shelf-brick-pak.zip`. The
`card/test-shelfpak.sh` script tests the pak's launch script off the device.

---

## Credits and licence

Shelf is free software under the GPL-3.0-or-later; see [LICENSE](LICENSE).

It is based on [slot](https://github.com/BrandonKowalski/slot) by Brandon T.
Kowalski, copyright 2026 Brandon T. Kowalski, which began as a Game Boy frontend
for Anbernic handhelds. The cart and shelf drawing, the cart sounds and the
sticker owe to that work. Type is Open Sans and Nerd Fonts Symbols by Ryan L
McIntyre.

The Rust code was put together with Claude and reviewed by hand.
