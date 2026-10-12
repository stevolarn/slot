#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/tg5040/Shelf.pak/launch.sh"
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

# Stands in for shelf: each run hands off the next line of $TMP/picks, or
# exits with nothing handed off once they run out.
cat > "$TMP/fake-shelf" <<'FAKE'
#!/bin/sh
n="$(cat "$PICKS.n" 2>/dev/null || echo 0)"
n=$((n + 1))
echo "$n" > "$PICKS.n"
line="$(sed -n "${n}p" "$PICKS")"
[ -n "$line" ] && echo "$line" > "$SHELF_HANDOFF"
echo "roms=$SHELF_ROMS root=$SHELF_ROOT" >> "$PICKS.env"
exit 0
FAKE
chmod +x "$TMP/fake-shelf"

run() {
	rm -f "$TMP/played" "$TMP/picks.n" "$TMP/picks.env"
	PICKS="$TMP/picks" SDCARD_PATH="$SD" SYSTEM_PATH="$SYS" PLATFORM=tg5040 \
		USERDATA_PATH="$SD/.userdata/tg5040" ROMS_PATH="$SD/Roms" \
		SHELF_PAK="$TMP/pak" SHELF_NEXT="$TMP/next" SHELF_RUN="$TMP/fake-shelf" \
		sh "$SCRIPT" > /dev/null 2>&1
}

fail() {
	echo "test-shelfpak: $*" >&2
	exit 1
}

printf '%s\n' "$SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba" \
	"$SD/Roms/Game Boy (GB)/Metroid II.gb" > "$TMP/picks"
# A card from before the rename has its data in .userdata/tg5040/slot.
mkdir -p "$SD/.userdata/tg5040/slot/Carts"
: > "$SD/.userdata/tg5040/slot/Carts/Metroid II.png"
run
[ -f "$SD/.userdata/tg5040/Shelf/Carts/Metroid II.png" ] || fail "the old slot data folder was not moved to Shelf"
[ ! -e "$SD/.userdata/tg5040/slot" ] || fail "the old slot data folder was left behind"
expected="GBA $SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba
GB $SD/Roms/Game Boy (GB)/Metroid II.gb"
[ "$(cat "$TMP/played")" = "$expected" ] || fail "games were not run through their emulator paks: $(cat "$TMP/played")"
[ "$(cat "$TMP/picks.n")" = 3 ] || fail "shelf was not started again after each game"
grep -q "roms=$SD/Roms root=$SD/.userdata/tg5040/Shelf" "$TMP/picks.env" || fail "shelf was not pointed at Roms and its data folder"

mkdir -p "$SD/Roms/Game Boy (GB)/00) All but the Best (Gameboy)"
: > "$SD/Roms/Game Boy (GB)/00) All but the Best (Gameboy)/Wario Land.gb"
printf '%s\n' "$SD/Roms/Game Boy (GB)/00) All but the Best (Gameboy)/Wario Land.gb" > "$TMP/picks"
run
[ "$(cat "$TMP/played" 2>/dev/null)" = "GB $SD/Roms/Game Boy (GB)/00) All but the Best (Gameboy)/Wario Land.gb" ] ||
	fail "a game in a subfolder did not go through its system's emulator pak"

printf '%s\n' "$SD/Roms/Arcade (FBN)/Metal Slug.zip" > "$TMP/picks"
run
[ ! -f "$TMP/played" ] || fail "a game with no emulator pak was run"
[ "$(cat "$TMP/picks.n")" = 2 ] || fail "shelf did not come back after a game with no emulator pak"

: > "$TMP/picks"
run
[ ! -f "$TMP/played" ] || fail "quitting shelf ran a game"
[ "$(cat "$TMP/picks.n")" = 1 ] || fail "shelf was started again after it quit"

echo "test-shelfpak: ok"
