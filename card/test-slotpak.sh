#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/tg5040/Slot.pak/launch.sh"
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
# "rom|resume" hands off the rom, then a "resume" line.
[ -n "$line" ] && echo "$line" | tr '|' '\n' > "$SHELF_HANDOFF"
echo "roms=$SHELF_ROMS root=$SHELF_ROOT" >> "$PICKS.env"
exit 0
FAKE
chmod +x "$TMP/fake-shelf"

run() {
	rm -f "$TMP/played" "$TMP/picks.n" "$TMP/picks.env"
	PICKS="$TMP/picks" SDCARD_PATH="$SD" SYSTEM_PATH="$SYS" PLATFORM=tg5040 \
		USERDATA_PATH="$SD/.userdata/tg5040" ROMS_PATH="$SD/Roms" \
		SHARED_USERDATA_PATH="$SD/.userdata/shared" SHELF_RESUME_SLOT="$TMP/resume_slot.txt" \
		SHELF_PAK="$TMP/pak" SHELF_NEXT="$TMP/next" SHELF_RUN="$TMP/fake-shelf" SHELF_RAN="$TMP/ran" \
		sh "$SCRIPT" > "$TMP/run.log" 2>&1
}

fail() {
	[ -z "${SHOW_LOG:-}" ] || cat "$TMP/run.log" >&2
	echo "test-slotpak: $*" >&2
	exit 1
}

printf '%s\n' "$SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba" \
	"$SD/Roms/Game Boy (GB)/Metroid II.gb" > "$TMP/picks"
# Another pak's data folder and boot hooks (Shelf's) are never touched.
mkdir -p "$SD/.userdata/tg5040/Shelf/Carts" "$SD/.userdata/tg5040/.hooks/boot.d"
: > "$SD/.userdata/tg5040/Shelf/Carts/Metroid II.png"
: > "$SD/.userdata/tg5040/.hooks/boot.d/shelf-resume.sync.sh"
run
[ -f "$SD/.userdata/tg5040/Shelf/Carts/Metroid II.png" ] || fail "Shelf's data folder was moved or removed"
[ -f "$SD/.userdata/tg5040/.hooks/boot.d/shelf-resume.sync.sh" ] || fail "Shelf's boot hook was removed"
[ ! -e "$SD/.userdata/tg5040/Slot/Carts/Metroid II.png" ] || fail "Shelf's data was taken into Slot"
expected="GBA $SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba
GB $SD/Roms/Game Boy (GB)/Metroid II.gb"
[ "$(cat "$TMP/played")" = "$expected" ] || fail "games were not run through their emulator paks: $(cat "$TMP/played")"
[ "$(cat "$TMP/picks.n")" = 3 ] || fail "shelf was not started again after each game"
grep -q "roms=$SD/Roms root=$SD/.userdata/tg5040/Slot" "$TMP/picks.env" || fail "shelf was not pointed at Roms and its data folder"

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

# NextUI's Resume: a "resume" second line plus the game's slot file puts
# the slot in the resume file; otherwise nothing is left there.
gba="$SD/Roms/Game Boy Advance (GBA)/Minish Cap.gba"
slots="$SD/.userdata/shared/.minui/GBA"
mkdir -p "$slots"
printf '2' > "$slots/Minish Cap.gba.txt"
printf '%s\n' "$gba|resume" > "$TMP/picks"
run
[ "$(cat "$TMP/resume_slot.txt" 2>/dev/null)" = 2 ] || fail "resume with a slot file did not write the slot"
[ "$(cat "$TMP/played")" = "GBA $gba" ] || fail "a resumed game was not run through its emulator pak"

printf '%s\n' "$gba" > "$TMP/picks"
run
[ ! -e "$TMP/resume_slot.txt" ] || fail "a game played without resume left a resume file (stale one not removed)"

rm "$slots/Minish Cap.gba.txt"
printf '%s\n' "$gba|resume" > "$TMP/picks"
run
[ ! -e "$TMP/resume_slot.txt" ] || fail "resume with no slot file wrote a resume file"
[ "$(cat "$TMP/played")" = "GBA $gba" ] || fail "a game with no save slot did not start"

# A game's emulator chosen in Slot's Start menu runs on that pak, if it is
# installed; one that is not falls back to the Roms folder's pak.
mkdir -p "$SD/Emus/tg5040/MGBA.pak"
cat > "$SD/Emus/tg5040/MGBA.pak/launch.sh" <<PAK
#!/bin/sh
echo "MGBA \$1" >> "$TMP/played"
PAK
chmod +x "$SD/Emus/tg5040/MGBA.pak/launch.sh"
mkdir -p "$SD/.userdata/tg5040/Slot/Config"
printf 'MGBA\t%s\n' "$gba" > "$SD/.userdata/tg5040/Slot/Config/emus.txt"
printf '%s\n' "$gba" > "$TMP/picks"
run
[ "$(cat "$TMP/played")" = "MGBA $gba" ] || fail "a game with a chosen emulator did not run on it: $(cat "$TMP/played")"
printf 'NOPE\t%s\n' "$gba" > "$SD/.userdata/tg5040/Slot/Config/emus.txt"
run
[ "$(cat "$TMP/played")" = "GBA $gba" ] || fail "a missing chosen emulator did not fall back to the folder's pak"
rm -f "$SD/.userdata/tg5040/Slot/Config/emus.txt"
run
[ "$(cat "$TMP/played")" = "GBA $gba" ] || fail "a game with no choice did not run on its folder's pak"

# Open at Power-On: the hook launch.sh leaves in boot.d runs Slot's own
# launch.sh with SHELF_BOOT set, only with boot=1, fewer than two starts
# left unfinished, and Slot not already run this boot.
HOOK="$SD/.userdata/tg5040/.hooks/boot.d/slot-start.sync.sh"
[ -x "$HOOK" ] || fail "no power-on hook was written"
CONF="$SD/.userdata/tg5040/Slot/Config"
mkdir -p "$CONF"
cat > "$TMP/pak/launch.sh" <<'STUB'
#!/bin/sh
echo "SHELF_BOOT=$SHELF_BOOT" >> "$STARTED"
STUB
hook() {
	rm -f "$TMP/started" "$TMP/ran"
	[ -z "${RAN_ALREADY:-}" ] || : > "$TMP/ran"
	STARTED="$TMP/started" sh "$HOOK" > /dev/null 2>&1
}
printf 'boot=0\n' > "$CONF/slot.state"
rm -f "$CONF/boot-tries"
hook
[ ! -e "$TMP/started" ] || fail "Slot opened at power-on with the setting off"
printf 'cart=Emerald\nboot=1\n' > "$CONF/slot.state"
hook
[ "$(cat "$TMP/started" 2>/dev/null)" = "SHELF_BOOT=1" ] || fail "Slot did not open at power-on with the setting on"
[ "$(cat "$CONF/boot-tries")" = 1 ] || fail "a start at power-on was not counted"
hook
[ -e "$TMP/started" ] || fail "one unfinished start stopped Slot opening"
[ "$(cat "$CONF/boot-tries")" = 2 ] || fail "the second start was not counted"
hook
[ ! -e "$TMP/started" ] || fail "Slot opened after two starts that never drew"
rm -f "$CONF/boot-tries"
RAN_ALREADY=1 hook
[ ! -e "$TMP/started" ] || fail "Slot opened a second time in one boot"
mkdir -p "$SD/.userdata/shared/.minui"
printf '/Roms/Game Boy (GB)/Metroid II.gb\n' > "$SD/.userdata/shared/.minui/auto_resume.txt"
SHARED_USERDATA_PATH="$SD/.userdata/shared" hook
[ ! -e "$TMP/started" ] || fail "Slot opened over a game NextUI had to resume"
rm -f "$SD/.userdata/shared/.minui/auto_resume.txt"
SHARED_USERDATA_PATH="$SD/.userdata/shared" hook
[ -e "$TMP/started" ] || fail "a resume note left behind kept Slot from opening"

echo "test-slotpak: ok"
