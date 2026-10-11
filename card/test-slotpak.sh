#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/tg5040/slot.pak/launch.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

SD="$TMP/sd"
SYS="$SD/.system/tg5040"
mkdir -p "$SD/Roms/Game Boy Advance (GBA)" "$SD/Roms/Game Boy (GB)" "$SYS/paks/Emus/GBA.pak" \
	"$SD/Emus/tg5040/GB.pak" "$TMP/pak"
: > "$SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba"
: > "$SD/Roms/Game Boy (GB)/Metroid II.gb"

for tag in GBA GB; do
	case "$tag" in
	GBA) dir="$SYS/paks/Emus/GBA.pak" ;;
	GB) dir="$SD/Emus/tg5040/GB.pak" ;;
	esac
	cat > "$dir/launch.sh" <<PAK
#!/bin/sh
echo "$tag \$1" >> "$TMP/played"
PAK
	chmod +x "$dir/launch.sh"
done

# Stands in for slot: each run hands off the next line of $TMP/picks, or
# exits with nothing handed off once they run out.
cat > "$TMP/fake-slot" <<'FAKE'
#!/bin/sh
n="$(cat "$PICKS.n" 2>/dev/null || echo 0)"
n=$((n + 1))
echo "$n" > "$PICKS.n"
line="$(sed -n "${n}p" "$PICKS")"
[ -n "$line" ] && echo "$line" > "$SLOT_HANDOFF"
echo "roms=$SLOT_ROMS root=$SLOT_ROOT" >> "$PICKS.env"
exit 0
FAKE
chmod +x "$TMP/fake-slot"

run() {
	rm -f "$TMP/played" "$TMP/picks.n" "$TMP/picks.env"
	PICKS="$TMP/picks" SDCARD_PATH="$SD" SYSTEM_PATH="$SYS" PLATFORM=tg5040 \
		USERDATA_PATH="$SD/.userdata/tg5040" ROMS_PATH="$SD/Roms" \
		SLOT_PAK="$TMP/pak" SLOT_NEXT="$TMP/next" AGS_SLOT_RUN="$TMP/fake-slot" \
		sh "$SCRIPT" > /dev/null 2>&1
}

fail() {
	echo "test-slotpak: $*" >&2
	exit 1
}

printf '%s\n' "$SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba" \
	"$SD/Roms/Game Boy (GB)/Metroid II.gb" > "$TMP/picks"
run
expected="GBA $SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba
GB $SD/Roms/Game Boy (GB)/Metroid II.gb"
[ "$(cat "$TMP/played")" = "$expected" ] || fail "games were not run through their emulator paks: $(cat "$TMP/played")"
[ "$(cat "$TMP/picks.n")" = 3 ] || fail "slot was not started again after each game"
grep -q "roms=$SD/Roms root=$SD/.userdata/tg5040/slot" "$TMP/picks.env" || fail "slot was not pointed at Roms and its data folder"

printf '%s\n' "$SD/Roms/Arcade (FBN)/Metal Slug.zip" > "$TMP/picks"
run
[ ! -f "$TMP/played" ] || fail "a game with no emulator pak was run"
[ "$(cat "$TMP/picks.n")" = 2 ] || fail "slot did not come back after a game with no emulator pak"

: > "$TMP/picks"
run
[ ! -f "$TMP/played" ] || fail "quitting slot ran a game"
[ "$(cat "$TMP/picks.n")" = 1 ] || fail "slot was started again after it quit"

echo "test-slotpak: ok"
