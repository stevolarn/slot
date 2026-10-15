#!/bin/sh
# Put Slot on a TrimUI Brick that runs NextUI, either on its SD card in a
# card reader or straight onto the Brick over the network.
#
#   card/tg5040/install-sd.sh /media/$USER/SDCARD [slot-brick-pak.zip]
#   card/tg5040/install-sd.sh ftp://192.168.1.231/ [slot-brick-pak.zip]
#
# The target is a folder where the card is mounted, or an ftp:// or
# sftp:// address whose top folder is the card's. Over the network it uses
# curl. A login, if the Brick wants one, comes from ~/.netrc or from
# SHELF_LOGIN=user:password; neither is stored by this script.
#
# Copies the pak to Tools/tg5040/Slot.pak, and the real cart photos,
# labels, box backs and game details (written by tools/details.py from the
# LaunchBox metadata) from the laptop test setup to .userdata/tg5040/Slot,
# shrunk with ImageMagick, when it is there, to the size Slot draws them.
# The shrunk copies are kept in ~/.cache/slot-brick-art; SHELF_FULL_ART=1
# sends the pictures as they are.
#
# With SHELF_ROMS_FROM set to a Roms folder laid out the NextUI way (one
# "Name (TAG)" folder per system, Top 100 lists in _Collections/Top 100),
# it also copies the Top 100 lists and only the games they name to the
# card's Roms folder, each with its box picture from the .media folder
# beside it. Satellaview is left out. The source is only read.
#
# Art and games the card already has, at the same size, are skipped, so a
# second run only sends what is new. SHELF_FORCE=1 copies everything again.
# The pak itself is always copied.
#
echo "pak: $PAK_ZIP -> $SD/Tools/tg5040/Slot.pak"
if [ -n "$REMOTE" ]; then
	unpacked="$(mktemp -d)"
	unzip -oq "$PAK_ZIP" -d "$unpacked"
	put_tree "$unpacked/Tools/tg5040/Slot.pak" "Tools/tg5040/Slot.pak" put_whole
else
	rm -rf "$SD/Tools/tg5040/Slot.pak"
	unzip -oq "$PAK_ZIP" -d "$SD"
	[ -x "$SD/Tools/tg5040/Slot.pak/slot" ] || chmod +x "$SD/Tools/tg5040/Slot.pak/slot" 2>/dev/null || true
fi

# How big Slot ever draws each kind of picture, for ImageMagick: a cart
# photo is trimmed to the cart and drawn at most 360 by 337, a label is
# pasted on a cart smaller than that, and a box scan (front or back) is
# drawn at most 600 by 600, on the box shelf. Sending them any bigger only
# makes the Brick read and shrink pixels it throws away.
slim_args() {
	case "$1" in
	Carts) echo "-trim +repage -resize 360x360>" ;;
	Labels) echo "-resize 400x400>" ;;
	Boxes) echo "-resize 600x600>" ;;
	esac
}

# Where the shrunk copies of a folder are kept: by size, so a new size makes
# new copies rather than reusing smaller ones.
slim_dir() {
	case "$1" in
	Boxes) echo "Boxes-600" ;;
	*) echo "$1" ;;
	esac
}

# slim <folder name>: shrunk copies of that art folder's pictures, made once
# and kept in SLIM, so later runs only shrink what is new. Prints the folder
# to send. Without ImageMagick, or with SHELF_FULL_ART=1, the art as it is.
SLIM="${SHELF_SLIM:-${XDG_CACHE_HOME:-$HOME/.cache}/slot-brick-art}"
slim() {
	if [ -n "${SHELF_FULL_ART:-}" ] || ! command -v convert > /dev/null; then
		echo "$ART/$1"
		return
	fi
	made=0
	(cd "$ART/$1" && find . -type f) > "$LISTINGS/slim-$1"
	while IFS= read -r f; do
		f="${f#./}"
		src="$ART/$1/$f"
		out="$SLIM/$(slim_dir "$1")/$f"
		[ -f "$out" ] && [ ! "$src" -nt "$out" ] && continue
		mkdir -p "$(dirname "$out")"
		case "$f" in
		*.png)
			# Through a pipe, so a [ in a game's name is not read as
			# ImageMagick's frame syntax.
			# shellcheck disable=SC2046 # the arguments are split on purpose
			if convert png:- $(slim_args "$1") -strip png:- < "$src" > "$out.tmp" 2> /dev/null; then
				mv -f "$out.tmp" "$out"
			else
				rm -f "$out.tmp"
				cp "$src" "$out"
			fi
			made=$((made + 1))
			[ $((made % 50)) -eq 0 ] && echo "  shrunk $made $1 pictures" >&2
			;;
		*) cp "$src" "$out" ;;
		esac
	done < "$LISTINGS/slim-$1"
	echo "$SLIM/$(slim_dir "$1")"
}

for dir in Carts Labels Boxes Details; do
	if [ -d "$ART/$dir" ]; then
		from="$(slim "$dir")"
		echo "$dir: $from -> $SD/.userdata/tg5040/Slot/$dir"
		put_tree "$from" ".userdata/tg5040/Slot/$dir" put_new
	else
		echo "$dir: none at $ART/$dir, skipped"
	fi
done

if [ -n "$ROMS_FROM" ]; then
	LISTS="$ROMS_FROM/_Collections/Top 100"
	if [ ! -d "$LISTS" ]; then
		echo "no Top 100 lists at $LISTS" >&2
		exit 1
	fi
	copied=0
	skipped=0
	missing=0
	art=0
	for list in "$LISTS"/*.txt; do
		[ -f "$list" ] || continue
		case "$list" in
		*[Ss]atellaview*) continue ;;
		esac
		put "$list" "Roms/_Collections/Top 100/$(basename "$list")"
		# The count is kept in a file because the loop runs in a subshell.
		tr -d '\r' < "$list" | while IFS= read -r line || [ -n "$line" ]; do
			rel="${line#/Roms/}"
			[ -n "$rel" ] && [ "$rel" != "$line" ] || continue
			if [ ! -f "$ROMS_FROM/$rel" ]; then
				echo "  missing: $rel" >&2
				echo missing
			elif already_there "$ROMS_FROM/$rel" "Roms/$rel" < /dev/null; then
				echo skipped
			else
				put "$ROMS_FROM/$rel" "Roms/$rel" < /dev/null
				echo copied
			fi
			# Its box picture, which NextUI and Slot's box view both read
			# from the .media folder beside the game.
			name="$(basename "$rel")"
			media="$(dirname "$rel")/.media/${name%.*}.png"
			if [ -f "$ROMS_FROM/$media" ]; then
				if already_there "$ROMS_FROM/$media" "Roms/$media" < /dev/null; then
					echo art-skipped
				else
					put "$ROMS_FROM/$media" "Roms/$media" < /dev/null
					echo art-copied
				fi
			fi
		done > "${TMPDIR:-/tmp}/shelf-install-$$"
		copied=$((copied + $(grep -c '^copied$' "${TMPDIR:-/tmp}/shelf-install-$$" || true)))
		skipped=$((skipped + $(grep -c '^skipped$' "${TMPDIR:-/tmp}/shelf-install-$$" || true)))
		missing=$((missing + $(grep -c '^missing$' "${TMPDIR:-/tmp}/shelf-install-$$" || true)))
		art=$((art + $(grep -c '^art-copied$' "${TMPDIR:-/tmp}/shelf-install-$$" || true)))
		rm -f "${TMPDIR:-/tmp}/shelf-install-$$"
		echo "Top 100: $(basename "$list")"
	done
	echo "Top 100 games: $copied copied, $skipped already there, $missing missing"
	echo "Box pictures: $art copied"
fi

if [ -n "$REMOTE" ]; then
	echo "done. On the Brick, open Tools > Slot."
	exit 0
fi

found=no
for only in "$SD/Collections/Top 100" "$SD/Roms/_Collections/Top 100" "$SD/_Collections/Top 100"; do
	if [ -d "$only" ]; then
		echo "Top 100 lists: $only (Slot will show only these games)"
		found=yes
	fi
done
[ "$found" = yes ] || echo "no Top 100 lists on the card, so Slot will show every game"

echo "syncing..."
sync
echo "done. Eject the card, put it in the Brick and open Tools > Slot."
