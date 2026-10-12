#!/bin/sh
# Put Shelf on a TrimUI Brick that runs NextUI, either on its SD card in a
# card reader or straight onto the Brick over the network.
#
#   card/tg5040/install-sd.sh /media/$USER/SDCARD [shelf-brick-pak.zip]
#   card/tg5040/install-sd.sh ftp://192.168.1.231/ [shelf-brick-pak.zip]
#
# The target is a folder where the card is mounted, or an ftp:// or
# sftp:// address whose top folder is the card's. Over the network it uses
# curl. A login, if the Brick wants one, comes from ~/.netrc or from
# SHELF_LOGIN=user:password; neither is stored by this script.
#
# Copies the pak to Tools/tg5040/Shelf.pak, and the real cart photos,
# labels, box backs and game details (written by tools/details.py from the
# LaunchBox metadata) from the laptop test setup to .userdata/tg5040/Shelf,
# shrunk with ImageMagick, when it is there, to the size Shelf draws them.
# The shrunk copies are kept in ~/.cache/shelf-brick-art; SHELF_FULL_ART=1
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
# Shelf used to be called slot. The old names of the settings below
# (SLOT_LOGIN, SLOT_ROMS_FROM, SLOT_FORCE, SLOT_FULL_ART, SLOT_ART,
# SLOT_SLIM) still work. A card with the old .userdata/tg5040/slot folder
# has it renamed to Shelf, which keeps its art and settings. A card with the
# old Tools/tg5040/slot.pak keeps it, and a note says to delete it; with
# SHELF_REMOVE_OLD=1 a card in a reader has it deleted (over the network
# it is only ever mentioned).
#
# Nothing else on the card is touched, and nothing is deleted.
set -eu

SHELF_LOGIN="${SHELF_LOGIN:-${SLOT_LOGIN:-}}"
SHELF_ROMS_FROM="${SHELF_ROMS_FROM:-${SLOT_ROMS_FROM:-}}"
SHELF_FORCE="${SHELF_FORCE:-${SLOT_FORCE:-}}"
SHELF_FULL_ART="${SHELF_FULL_ART:-${SLOT_FULL_ART:-}}"
SHELF_ART="${SHELF_ART:-${SLOT_ART:-}}"
SHELF_SLIM="${SHELF_SLIM:-${SLOT_SLIM:-}}"

SD="${1:-}"
PAK_ZIP="${2:-$HOME/Downloads/shelf-brick-pak.zip}"
ROMS_FROM="${SHELF_ROMS_FROM:-}"
# The laptop test setup's art: Shelf's folder, or slot's if it has no other.
if [ -z "$SHELF_ART" ]; then
	SHELF_ART="$HOME/slot-picker-test/.userdata/tg5040/Shelf"
	[ -d "$SHELF_ART" ] || SHELF_ART="$HOME/slot-picker-test/.userdata/tg5040/slot"
fi
ART="$SHELF_ART"

usage() {
	echo "usage: $0 <SD card folder, or ftp://brick-address/> [shelf-brick-pak.zip]" >&2
	echo "eg /media/${USER:-you}/SDCARD or ftp://192.168.1.231/" >&2
	exit 1
}

case "$SD" in
ftp://* | sftp://*)
	REMOTE="${SD%/}"
	SD="$REMOTE"
	;;
"") usage ;;
*)
	REMOTE=""
	[ -d "$SD" ] || usage
	;;
esac

# The path part of an address, with the characters a URL cannot carry
# as they are written out.
encode() {
	printf '%s' "$1" | sed -e 's/%/%25/g' -e 's/ /%20/g' -e 's/#/%23/g' -e 's/?/%3F/g' \
		-e 's/\[/%5B/g' -e 's/\]/%5D/g'
}

# How curl talks to the Brick: picked below from what its server accepts.
MODE=""

curl_to() {
	# shellcheck disable=SC2086 # MODE is a list of options
	if [ -n "${SHELF_LOGIN:-}" ]; then
		curl -sS --fail --globoff --connect-timeout 10 --ftp-create-dirs $MODE -u "$SHELF_LOGIN" "$@"
	else
		curl -sS --fail --globoff --connect-timeout 10 --ftp-create-dirs $MODE --netrc-optional "$@"
	fi
}

# put <file> <path on the card>
put() {
	if [ -n "$REMOTE" ]; then
		# The Brick's FTP server now and then drops a connection, so give
		# each file a few tries before giving up.
		tries=0
		until curl_to -T "$1" "$REMOTE/$(encode "$2")"; do
			tries=$((tries + 1))
			if [ "$tries" -ge 4 ]; then
				echo "could not copy $2 to the Brick" >&2
				return 1
			fi
			echo "  retrying $2" >&2
			sleep "$tries"
		done
	else
		mkdir -p "$SD/$(dirname "$2")"
		cp "$1" "$SD/$2"
	fi
}

# Remote folder listings, fetched once per folder.
LISTINGS="$(mktemp -d)"
trap 'rm -rf "$LISTINGS" "${unpacked:-}"' EXIT

# already_there <file> <path on the card>: the card has a file of the same
# size at that path.
already_there() {
	[ -z "${SHELF_FORCE:-}" ] || return 1
	want="$(wc -c < "$1" | tr -d ' ')"
	if [ -z "$REMOTE" ]; then
		[ -f "$SD/$2" ] && [ "$(wc -c < "$SD/$2" | tr -d ' ')" = "$want" ]
		return
	fi
	listing="$LISTINGS/$(dirname "$2" | cksum | cut -d' ' -f1)"
	if [ ! -f "$listing" ]; then
		curl_to "$REMOTE/$(encode "$(dirname "$2")")/" > "$listing" 2> /dev/null < /dev/null || : > "$listing"
	fi
	# An ls -l style line: the name follows the first eight fields.
	tr -d '\r' < "$listing" | awk -v n="$(basename "$2")" -v s="$want" '
		{ name = $0; for (i = 1; i <= 8; i++) sub(/^ *[^ ]+ +/, "", name) }
		name == n && $5 == s { found = 1 }
		END { exit !found }'
}

# put_whole <file> <path on the card>: put, but over the network send it to
# a temporary name first and rename it into place once it has all arrived,
# so a connection the Brick drops part way never leaves half a program on
# the card. (Paths here have no spaces.)
put_whole() {
	if [ -z "$REMOTE" ]; then
		put "$1" "$2"
		return
	fi
	put "$1" "$2.part" || return 1
	curl_to -Q "-RNFR $2.part" -Q "-RNTO $2" "$REMOTE/" > /dev/null 2>&1 < /dev/null && return 0
	# Some servers will not rename over a file: remove the old one first.
	curl_to -Q "-DELE $2" -Q "-RNFR $2.part" -Q "-RNTO $2" "$REMOTE/" > /dev/null 2>&1 < /dev/null && return 0
	echo "could not move $2.part into place on the Brick" >&2
	return 1
}

# put_new <file> <path on the card>: put, unless the card already has it.
put_new() {
	already_there "$1" "$2" || put "$1" "$2"
}

# put_tree <folder> <path on the card> [put command]: every file under the
# folder.
put_tree() {
	(cd "$1" && find . -type f) | while IFS= read -r f; do
		"${3:-put}" "$1/${f#./}" "$2/${f#./}" < /dev/null
	done
}

if [ -n "$REMOTE" ]; then
	command -v curl > /dev/null || {
		echo "curl is needed to copy over the network" >&2
		exit 1
	}
	# Small FTP servers often refuse curl's default passive mode, so try
	# the older passive command, then active mode, before giving up.
	top=""
	for MODE in "" "--disable-epsv" "--ftp-port -" "--disable-epsv --ftp-pasv --ftp-skip-pasv-ip"; do
		if top="$(curl_to --list-only "$REMOTE/" 2> "${TMPDIR:-/tmp}/shelf-curl-$$")"; then
			break
		fi
		top=""
	done
	if [ -z "$top" ] && grep -q -e '530' -e '(67)' "${TMPDIR:-/tmp}/shelf-curl-$$"; then
		rm -f "${TMPDIR:-/tmp}/shelf-curl-$$"
		echo "the Brick's FTP server wants a login: run again with SHELF_LOGIN=user:password in front" >&2
		exit 1
	fi
	if [ -z "$top" ]; then
		echo "could not list $REMOTE. curl said:" >&2
		cat "${TMPDIR:-/tmp}/shelf-curl-$$" >&2
		rm -f "${TMPDIR:-/tmp}/shelf-curl-$$"
		echo "if it asks for a login, run again with SHELF_LOGIN=user:password in front" >&2
		exit 1
	fi
	rm -f "${TMPDIR:-/tmp}/shelf-curl-$$"
	[ -z "$MODE" ] || echo "talking to the Brick with curl $MODE"
	if ! printf '%s\n' "$top" | grep -qx -e 'Roms' -e '.system'; then
		echo "$REMOTE has no Roms or .system folder, so it does not look like a NextUI card" >&2
		exit 1
	fi
elif [ ! -d "$SD/Roms" ] && [ ! -d "$SD/.system" ]; then
	echo "$SD has no Roms or .system folder, so it does not look like a NextUI card" >&2
	exit 1
fi
if [ ! -f "$PAK_ZIP" ]; then
	echo "no pak zip at $PAK_ZIP; download shelf-brick-pak.zip or pass its path" >&2
	exit 1
fi

# on_card <folder>: the card has that folder (a path under its top folder).
on_card() {
	if [ -n "$REMOTE" ]; then
		curl_to --list-only "$REMOTE/$(encode "$(dirname "$1")")/" 2> /dev/null < /dev/null |
			tr -d '\r' | grep -qx "$(basename "$1")"
	else
		[ -d "$SD/$1" ]
	fi
}

# Shelf used to be called slot: its data folder on the card, with the art
# and settings in it, is renamed to Shelf rather than started over.
if on_card .userdata/tg5040/slot && ! on_card .userdata/tg5040/Shelf; then
	echo "renaming the old .userdata/tg5040/slot to .userdata/tg5040/Shelf"
	if [ -n "$REMOTE" ]; then
		curl_to -Q "-RNFR .userdata/tg5040/slot" -Q "-RNTO .userdata/tg5040/Shelf" \
			"$REMOTE/" > /dev/null 2>&1 < /dev/null ||
			echo "could not rename it: rename .userdata/tg5040/slot to Shelf on the card by hand" >&2
	else
		mv "$SD/.userdata/tg5040/slot" "$SD/.userdata/tg5040/Shelf"
	fi
fi

# The old pak is never deleted without being asked to.
old_pak_note() {
	on_card Tools/tg5040/slot.pak || return 0
	if [ -z "$REMOTE" ] && [ "${SHELF_REMOVE_OLD:-}" = 1 ]; then
		rm -rf "$SD/Tools/tg5040/slot.pak"
		echo "deleted the old Tools/tg5040/slot.pak"
	else
		echo "note: the card still has the old Tools/tg5040/slot.pak. Delete it so only" >&2
		echo "Shelf shows in Tools (or run again with SHELF_REMOVE_OLD=1 on a card in a reader)." >&2
	fi
}

echo "pak: $PAK_ZIP -> $SD/Tools/tg5040/Shelf.pak"
if [ -n "$REMOTE" ]; then
	unpacked="$(mktemp -d)"
	unzip -oq "$PAK_ZIP" -d "$unpacked"
	put_tree "$unpacked/Tools/tg5040/Shelf.pak" "Tools/tg5040/Shelf.pak" put_whole
else
	rm -rf "$SD/Tools/tg5040/Shelf.pak"
	unzip -oq "$PAK_ZIP" -d "$SD"
	[ -x "$SD/Tools/tg5040/Shelf.pak/shelf" ] || chmod +x "$SD/Tools/tg5040/Shelf.pak/shelf" 2>/dev/null || true
fi

# How big Shelf ever draws each kind of picture, for ImageMagick: a cart
# photo is trimmed to the cart and drawn at most 360 by 337, a label is
# pasted on a cart smaller than that, and a box back is drawn at most 420
# by 300. Sending them any bigger only makes the Brick read and shrink
# pixels it throws away.
slim_args() {
	case "$1" in
	Carts) echo "-trim +repage -resize 360x360>" ;;
	Labels) echo "-resize 400x400>" ;;
	Boxes) echo "-resize 420x300>" ;;
	esac
}

# slim <folder name>: shrunk copies of that art folder's pictures, made once
# and kept in SLIM, so later runs only shrink what is new. Prints the folder
# to send. Without ImageMagick, or with SHELF_FULL_ART=1, the art as it is.
SLIM="${SHELF_SLIM:-${XDG_CACHE_HOME:-$HOME/.cache}/shelf-brick-art}"
# The shrunk copies slot made are as good as new ones: keep them.
OLD_SLIM="${XDG_CACHE_HOME:-$HOME/.cache}/slot-brick-art"
if [ -z "$SHELF_SLIM" ] && [ -d "$OLD_SLIM" ] && [ ! -e "$SLIM" ]; then
	mv "$OLD_SLIM" "$SLIM"
fi
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
		out="$SLIM/$1/$f"
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
	echo "$SLIM/$1"
}

for dir in Carts Labels Boxes Details; do
	if [ -d "$ART/$dir" ]; then
		from="$(slim "$dir")"
		echo "$dir: $from -> $SD/.userdata/tg5040/Shelf/$dir"
		put_tree "$from" ".userdata/tg5040/Shelf/$dir" put_new
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
			# Its box picture, which NextUI and Shelf's box view both read
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
	old_pak_note
	echo "done. On the Brick, open Tools > Shelf."
	exit 0
fi

found=no
for only in "$SD/Collections/Top 100" "$SD/Roms/_Collections/Top 100" "$SD/_Collections/Top 100"; do
	if [ -d "$only" ]; then
		echo "Top 100 lists: $only (Shelf will show only these games)"
		found=yes
	fi
done
[ "$found" = yes ] || echo "no Top 100 lists on the card, so Shelf will show every game"

echo "syncing..."
sync
old_pak_note
echo "done. Eject the card, put it in the Brick and open Tools > Shelf."
