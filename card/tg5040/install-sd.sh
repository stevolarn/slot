#!/bin/sh
# Put slot on a TrimUI Brick SD card that runs NextUI.
#
#   card/tg5040/install-sd.sh /media/$USER/SDCARD [slot-brick-pak.zip]
#
# Copies the pak to Tools/tg5040/slot.pak, and the real cart photos and
# labels from the laptop test setup to .userdata/tg5040/slot.
#
# With SLOT_ROMS_FROM set to a Roms folder laid out the NextUI way (one
# "Name (TAG)" folder per system, Top 100 lists in _Collections/Top 100),
# it also copies the Top 100 lists and only the games they name to the
# card's Roms folder. Satellaview is left out. The source is only read.
#
# Nothing else on the card is touched, and nothing is deleted.
set -eu

SD="${1:-}"
PAK_ZIP="${2:-$HOME/Downloads/slot-brick-pak.zip}"
ART="${SLOT_ART:-$HOME/slot-picker-test/.userdata/tg5040/slot}"
ROMS_FROM="${SLOT_ROMS_FROM:-}"

if [ -z "$SD" ] || [ ! -d "$SD" ]; then
	echo "usage: $0 <SD card folder> [slot-brick-pak.zip]" >&2
	echo "the SD card folder is where the card is mounted, eg /media/${USER:-you}/SDCARD" >&2
	exit 1
fi
if [ ! -d "$SD/Roms" ] && [ ! -d "$SD/.system" ]; then
	echo "$SD has no Roms or .system folder, so it does not look like a NextUI card" >&2
	exit 1
fi
if [ ! -f "$PAK_ZIP" ]; then
	echo "no pak zip at $PAK_ZIP; download slot-brick-pak.zip or pass its path" >&2
	exit 1
fi

echo "pak: $PAK_ZIP -> $SD/Tools/tg5040/slot.pak"
rm -rf "$SD/Tools/tg5040/slot.pak"
unzip -oq "$PAK_ZIP" -d "$SD"
[ -x "$SD/Tools/tg5040/slot.pak/slot" ] || chmod +x "$SD/Tools/tg5040/slot.pak/slot" 2>/dev/null || true

DATA="$SD/.userdata/tg5040/slot"
mkdir -p "$DATA"
for dir in Carts Labels; do
	if [ -d "$ART/$dir" ]; then
		echo "$dir: $ART/$dir -> $DATA/$dir"
		mkdir -p "$DATA/$dir"
		cp -R "$ART/$dir/." "$DATA/$dir/"
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
	mkdir -p "$SD/Roms/_Collections/Top 100"
	copied=0
	missing=0
	for list in "$LISTS"/*.txt; do
		[ -f "$list" ] || continue
		case "$list" in
		*[Ss]atellaview*) continue ;;
		esac
		cp "$list" "$SD/Roms/_Collections/Top 100/"
		while IFS= read -r line || [ -n "$line" ]; do
			line="$(printf '%s' "$line" | tr -d '\r')"
			rel="${line#/Roms/}"
			[ -n "$rel" ] && [ "$rel" != "$line" ] || continue
			if [ -f "$ROMS_FROM/$rel" ]; then
				mkdir -p "$SD/Roms/$(dirname "$rel")"
				cp "$ROMS_FROM/$rel" "$SD/Roms/$rel"
				copied=$((copied + 1))
			else
				echo "  missing: $rel"
				missing=$((missing + 1))
			fi
		done < "$list"
		echo "Top 100: $(basename "$list")"
	done
	echo "Top 100 games: $copied copied, $missing missing"
fi

found=no
for only in "$SD/Collections/Top 100" "$SD/Roms/_Collections/Top 100" "$SD/_Collections/Top 100"; do
	if [ -d "$only" ]; then
		echo "Top 100 lists: $only (slot will show only these games)"
		found=yes
	fi
done
[ "$found" = yes ] || echo "no Top 100 lists on the card, so slot will show every game"

echo "syncing..."
sync
echo "done. Eject the card, put it in the Brick and open Tools > slot."
