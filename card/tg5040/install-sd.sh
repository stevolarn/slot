#!/bin/sh
# Put slot on a TrimUI Brick that runs NextUI, either on its SD card in a
# card reader or straight onto the Brick over the network.
#
#   card/tg5040/install-sd.sh /media/$USER/SDCARD [slot-brick-pak.zip]
#   card/tg5040/install-sd.sh ftp://192.168.1.231/ [slot-brick-pak.zip]
#
# The target is a folder where the card is mounted, or an ftp:// or
# sftp:// address whose top folder is the card's. Over the network it uses
# curl. A login, if the Brick wants one, comes from ~/.netrc or from
# SLOT_LOGIN=user:password; neither is stored by this script.
#
# Copies the pak to Tools/tg5040/slot.pak, and the real cart photos,
# labels and box backs from the laptop test setup to .userdata/tg5040/slot,
# shrunk with ImageMagick, when it is there, to the size slot draws them.
# The shrunk copies are kept in ~/.cache/slot-brick-art; SLOT_FULL_ART=1
# sends the pictures as they are.
#
# With SLOT_ROMS_FROM set to a Roms folder laid out the NextUI way (one
# "Name (TAG)" folder per system, Top 100 lists in _Collections/Top 100),
# it also copies the Top 100 lists and only the games they name to the
# card's Roms folder, each with its box picture from the .media folder
# beside it. Satellaview is left out. The source is only read.
#
# Art and games the card already has, at the same size, are skipped, so a
# second run only sends what is new. SLOT_FORCE=1 copies everything again.
# The pak itself is always copied.
#
# Nothing else on the card is touched, and nothing is deleted.
set -eu

SD="${1:-}"
PAK_ZIP="${2:-$HOME/Downloads/slot-brick-pak.zip}"
ART="${SLOT_ART:-$HOME/slot-picker-test/.userdata/tg5040/slot}"
ROMS_FROM="${SLOT_ROMS_FROM:-}"

usage() {
	echo "usage: $0 <SD card folder, or ftp://brick-address/> [slot-brick-pak.zip]" >&2
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
	if [ -n "${SLOT_LOGIN:-}" ]; then
		curl -sS --fail --globoff --connect-timeout 10 --ftp-create-dirs $MODE -u "$SLOT_LOGIN" "$@"
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
	[ -z "${SLOT_FORCE:-}" ] || return 1
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
		if top="$(curl_to --list-only "$REMOTE/" 2> "${TMPDIR:-/tmp}/slot-curl-$$")"; then
			break
		fi
		top=""
	done
	if [ -z "$top" ] && grep -q -e '530' -e '(67)' "${TMPDIR:-/tmp}/slot-curl-$$"; then
		rm -f "${TMPDIR:-/tmp}/slot-curl-$$"
		echo "the Brick's FTP server wants a login: run again with SLOT_LOGIN=user:password in front" >&2
		exit 1
	fi
	if [ -z "$top" ]; then
		echo "could not list $REMOTE. curl said:" >&2
		cat "${TMPDIR:-/tmp}/slot-curl-$$" >&2
		rm -f "${TMPDIR:-/tmp}/slot-curl-$$"
		echo "if it asks for a login, run again with SLOT_LOGIN=user:password in front" >&2
		exit 1
	fi
	rm -f "${TMPDIR:-/tmp}/slot-curl-$$"
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
	echo "no pak zip at $PAK_ZIP; download slot-brick-pak.zip or pass its path" >&2
	exit 1
fi

echo "pak: $PAK_ZIP -> $SD/Tools/tg5040/slot.pak"
if [ -n "$REMOTE" ]; then
	unpacked="$(mktemp -d)"
	unzip -oq "$PAK_ZIP" -d "$unpacked"
	put_tree "$unpacked/Tools/tg5040/slot.pak" "Tools/tg5040/slot.pak"
else
	rm -rf "$SD/Tools/tg5040/slot.pak"
	unzip -oq "$PAK_ZIP" -d "$SD"
	[ -x "$SD/Tools/tg5040/slot.pak/slot" ] || chmod +x "$SD/Tools/tg5040/slot.pak/slot" 2>/dev/null || true
fi

# How big slot ever draws each kind of picture, for ImageMagick: a cart
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
# to send. Without ImageMagick, or with SLOT_FULL_ART=1, the art as it is.
SLIM="${SLOT_SLIM:-${XDG_CACHE_HOME:-$HOME/.cache}/slot-brick-art}"
slim() {
	if [ -n "${SLOT_FULL_ART:-}" ] || ! command -v convert > /dev/null; then
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

for dir in Carts Labels Boxes; do
	if [ -d "$ART/$dir" ]; then
		from="$(slim "$dir")"
		echo "$dir: $from -> $SD/.userdata/tg5040/slot/$dir"
		put_tree "$from" ".userdata/tg5040/slot/$dir" put_new
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
			# Its box picture, which NextUI and slot's box view both read
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
		done > "${TMPDIR:-/tmp}/slot-install-$$"
		copied=$((copied + $(grep -c '^copied$' "${TMPDIR:-/tmp}/slot-install-$$" || true)))
		skipped=$((skipped + $(grep -c '^skipped$' "${TMPDIR:-/tmp}/slot-install-$$" || true)))
		missing=$((missing + $(grep -c '^missing$' "${TMPDIR:-/tmp}/slot-install-$$" || true)))
		art=$((art + $(grep -c '^art-copied$' "${TMPDIR:-/tmp}/slot-install-$$" || true)))
		rm -f "${TMPDIR:-/tmp}/slot-install-$$"
		echo "Top 100: $(basename "$list")"
	done
	echo "Top 100 games: $copied copied, $skipped already there, $missing missing"
	echo "Box pictures: $art copied"
fi

if [ -n "$REMOTE" ]; then
	echo "done. On the Brick, open Tools > slot."
	exit 0
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
