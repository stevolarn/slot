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
		curl_to -T "$1" "$REMOTE/$(encode "$2")"
	else
		mkdir -p "$SD/$(dirname "$2")"
		cp "$1" "$SD/$2"
	fi
}

# put_tree <folder> <path on the card>: every file under the folder.
put_tree() {
	(cd "$1" && find . -type f) | while IFS= read -r f; do
		put "$1/${f#./}" "$2/${f#./}"
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
	trap 'rm -rf "$unpacked"' EXIT
	unzip -oq "$PAK_ZIP" -d "$unpacked"
	put_tree "$unpacked/Tools/tg5040/slot.pak" "Tools/tg5040/slot.pak"
else
	rm -rf "$SD/Tools/tg5040/slot.pak"
	unzip -oq "$PAK_ZIP" -d "$SD"
	[ -x "$SD/Tools/tg5040/slot.pak/slot" ] || chmod +x "$SD/Tools/tg5040/slot.pak/slot" 2>/dev/null || true
fi

for dir in Carts Labels Boxes; do
	if [ -d "$ART/$dir" ]; then
		echo "$dir: $ART/$dir -> $SD/.userdata/tg5040/slot/$dir"
		put_tree "$ART/$dir" ".userdata/tg5040/slot/$dir"
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
	missing=0
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
			if [ -f "$ROMS_FROM/$rel" ]; then
				put "$ROMS_FROM/$rel" "Roms/$rel" < /dev/null
				echo copied
			else
				echo "  missing: $rel" >&2
				echo missing
			fi
		done > "${TMPDIR:-/tmp}/slot-install-$$"
		copied=$((copied + $(grep -c '^copied$' "${TMPDIR:-/tmp}/slot-install-$$" || true)))
		missing=$((missing + $(grep -c '^missing$' "${TMPDIR:-/tmp}/slot-install-$$" || true)))
		rm -f "${TMPDIR:-/tmp}/slot-install-$$"
		echo "Top 100: $(basename "$list")"
	done
	echo "Top 100 games: $copied copied, $missing missing"
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
