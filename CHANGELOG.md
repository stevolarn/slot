# Changelog

The history before Shelf, back to the Anbernic frontend it began as, is in git.

## Unreleased

- The video store, larger: one big box in the middle on the shelf with an edge of the box either side, the aisle sign and the game's name up at the top. A opens the box, the console comes up from below and the cart comes out over its slot; A again slots it in and plays, B puts it back. B turns a box over where it stands. X swaps the store's shelf between boxes and loose carts (kept in `shelf.state` as `store_shelf=`). The tags under the boxes show plays instead of rental nights: NEW, RENTED 6×, and under the middle box the day it was last played; plays NextUI logged itself in `game_logs.sqlite` before Shelf took over are counted too. A game with a save has a CONTINUE? note tucked in its box in the store and on the box shelf.
- A box shelf display: Display now steps through Store, Carts and Boxes (L2, or left and right in Options). Boxes shows each game's box front large in the dark, filling the screen under its name, with just the edges of the boxes either side to scroll to, and a little shelf wear when Dusty Carts is on. B turns the box over to its back; A opens it and the cart comes out onto the cart shelf; A again plays it, B puts it back. A full-size scan of the front in `Boxes/<console>/<game>.front.png`, beside the backs, is drawn in place of the small NextUI picture, and the installer now sends box scans at 600 by 600.
- A video store display: L2 (or Menu, Options, Display) shows each console's games as one endless row of box fronts on a rental shop's shelf, in blue and yellow, with a hanging aisle sign, price tags and the console's colour along the rail. B opens the box to its back; A plays at once. The cart shelf stays the default.
- Sorting: R2 (or Options, Sort) orders each console's shelf by name, genre, year, publisher or rating, using the LaunchBox details. The sign (and a small tab on the cart shelf) names the section, and L and R jump a section at a time. Both settings are kept in `shelf.state`.
- Renamed to Shelf. The pak is `Tools/tg5040/Shelf.pak` and its data lives in `.userdata/tg5040/Shelf`; an old `slot` folder is renamed on first run, and settings in `slot.state` are read until `shelf.state` is written. The environment variables are `SHELF_*` now.
- Removed the emulator, the link cable, save states, shaders and Anbernic support. Shelf is a pak for NextUI on the TrimUI Brick, and NextUI's own emulators run the games.
- A pak for the Brick: Shelf hands the chosen game to NextUI's emulator pak and comes back to the shelf when the game quits.
- Blowing on a cart tips it up to show its bottom, with a breath over its contacts.
- A box back view: put a scan of the back of the box in `Boxes/` and B flips the box over.
- Art loads in the background, so the shelf no longer stalls while pictures are read.
- Smaller art: cart and label pictures are shrunk to the size they are drawn at, and the installer sends them that way.
- Box art is read ahead, so a box opens without waiting on the SD card.
