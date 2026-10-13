# Changelog

The history before Shelf, back to the Anbernic frontend it began as, is in git.

## Unreleased

- Coming back from a game prints a return receipt out of a till slot: the game, when it was RENTED and RETURNED, this rental's time and the running total, and a DUE BACK date three days on. It tears off after a few seconds or at any button. The original theme prints a grey session slip (STARTED, FINISHED, PLAYED). Options > Print Receipts turns it off.
- Every return is logged in `Config/receipts.txt`, printed or not, and the new Receipts menu row shows them on a spike, newest on top; Up and Down flip through them.
- About is now your membership card: member since, rentals, hours played and favourite genre on a laminated SHELF VIDEO card in the store, or a grey player card (PLAYS) in the original theme. A turns it over to the small print. Member since goes back to the first play Shelf or NextUI recorded, kept in `shelf.state` as `member_since=`.
- Get Artwork lists, before it starts, how many games on each console lack a box front, a box back and a cart photo (GG 156/156/156).
- Holding Left or Right on a shelf of 60 games or more goes three carts a step after about two seconds, so the far end of a long shelf is near.
- Menu with the volume buttons sets the brightness, as in NextUI (Select with Up and Down still does too). Menu now opens the menu when it is let go, so it can be held for this.
- X on the back of a box plays a saved game from its start instead of its last save state, as NextUI's own start button does. The save states are left as they are, and A still continues. Works in every theme and view.
- The original theme shows the plays under the name (PLAYED 3× · LAST 12 OCT, or NEW) for games LaunchBox has no details of too, as the store's tags do.
- Search, in the menu: spell a name a letter at a time as on an arcade's high score table. Up and Down pick the letter, Right adds another, B takes one back. The games whose names hold it are listed as you go, those starting with it first. L and R pick one, and A brings the shelf round to it. The menu's rows sit closer again to make room.
- Game Gear and Master System shelves, with carts of their own shape and slots in each console's colours. Get Artwork knows them as LaunchBox's Sega Game Gear and Sega Master System. A console the card's Top 100 lists name no game of now shows all its games, so the new shelves are not empty on a card with lists.
- Staff Picks (Picked for You in the original theme): a shelf beside Recently Rented that Shelf fills itself with up to ten games. First come games left with a save and not played for three days, then unplayed games in the genre played most, then the best rated games never played. The picks turn over each day.
- Play time: the middle box's tag in the store and the line under the name in the original theme add how long the game has been played (RENTED 7× · 4H 20M · LAST 12 OCT). Shelf times each game from going in to coming back, on the clock that stops while the Brick sleeps, in `Config/playtime.txt`, and adds the play time NextUI logged before.
- Hide Game, in the menu: takes the game in the middle off the shelves (kept in `Config/hidden.txt`). Options, Show Hidden puts hidden games back on the shelves, and Hide Game then reads Unhide Game over one of them. The menu's rows sit a little closer to fit it.
- Every console's shelf opens on the game it was left on, not only the one a game was played from: `shelf.state` keeps a `place_<console>=` line for each.

- Get Artwork, in the menu: fetches box fronts, box backs, cart photos and game details from the LaunchBox Games Database on the Brick itself, over Wi-Fi. LaunchBox's 108 MB list is read as it streams in and never saved; a small index of the card's games is kept so later runs go straight to the pictures. Box scans are saved as JPEG (Shelf now reads `.front.jpg` and `.back.jpg` as well as PNG), cart photos as trimmed PNG. The screen shows the room needed and free before it starts, a progress bar while it runs, and B stops it; it stops by itself when the card is nearly full or LaunchBox is busy, and keeps the Brick awake meanwhile.

- `tools/fetch_art.py`: fetches box fronts, box backs, cart photos and game details from the LaunchBox Games Database for the games on an SD card, shrunk to the size Shelf draws them.
- Themes: Options now has a Theme row (Original or Video Store) and a View row (Boxes or Carts), in place of Display. L2 swaps boxes and carts within the theme instead of stepping through all four displays. The original theme uses plain words: Favourites, Recently Played, PLAYED stickers, and MOST PLAYED, PLAYED OFTEN, NEVER PLAYED and TOP RATED sections.
- The two themes work alike. B on a loose cart in the store turns it round to its box, as on the cart shelf. The original theme prints the plays after the details under the name (PLAYED 7× · LAST 12 OCT, or NEW), as the store's tags do; shows the section on a tab over the name whatever the sort, as the store's sign does; and puts the CONTINUE? note on a saved cart in place of the SAVED tag. The button guide lists what the buttons do in the theme and view on show.

## 0.1.0

- Favourite Rentals: Y adds the game in the middle to a Favourite Rentals shelf, or takes it off again. The shelf sits above the first console, newest first, and is hidden while empty. The played shelf is now called Recently Rented.
- Start opens the button guide (it is also Button Guide in the menu); Menu opens the menu.
- In the store, A on a loose cart brings it out over the console, and A again slots it in; B puts it back.
- The store's footer suits the shop: the console's name on a yellow ticket, the battery as a copper-topped AA cell and the time in red VCR digits, centred between the strip's yellow lines, and kept while a cart is held over the console.
- The store's boxes and carts leave room under the header and stand just above the shelf's edge. NES and SNES cart photos at the sides no longer show a black block above them.
- Sorted by plays, the most played group is now TOP RENTALS.
- The pak carries the licences of the two fonts built into it.

- Shelf goes to sleep by itself after NextUI's screen timeout (`screentimeout` in `minuisettings.txt`) with no button pressed, as a tap on power does; set to never, it never does. A loose cart blown on in the store tips back to show its contacts, as on the cart shelf, and a cart taken out of its box, in the store or on the box shelf, can be blown on too.
- Sort by plays: R2 (or Options, Sort) now also orders a shelf by how often each game has been played, the most first, in sections of favourites, regulars, games rented once or twice, and new releases. By name every shelf is sectioned by single letters, so the sign and L and R go a letter at a time. In the store L2 shows the boxes, then loose carts, before the cart shelf; X blows on the box or cart; and A on a loose cart slots it straight into the console. The light behind the store's middle box is soft, and loose carts no longer show an outline round them.
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
