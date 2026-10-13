#!/bin/sh
# Shelf as a game picker for NextUI on the TrimUI Brick. Shelf shows its
# shelf, writes the chosen rom to $NEXT and exits; this loop runs that rom
# through NextUI's own emulator pak, the way NextUI's menu does, then
# starts Shelf again. Quitting Shelf without a game drops back to NextUI.
PAK="${SHELF_PAK:-$(cd "$(dirname "$0")" && pwd)}"
SD="${SDCARD_PATH:-/mnt/SDCARD}"
PLAT="${PLATFORM:-tg5040}"
SYS="${SYSTEM_PATH:-$SD/.system/$PLAT}"
# Shelf runs NextUI's suspend script for a tap of the power button, and the
# script finds its hooks through SYSTEM_PATH.
export SYSTEM_PATH="$SYS"
UD="${USERDATA_PATH:-$SD/.userdata/$PLAT}"
DATA="$UD/Shelf"
NEXT="${SHELF_NEXT:-/tmp/shelf-next}"
RUNNER="${SHELF_RUN:-/lib/ld-linux-aarch64.so.1}"
LOG="$DATA/probe.log"
# Shelf was called slot: keep its Carts, Labels, Boxes and Config.
if [ ! -d "$DATA" ] && [ -d "$UD/slot" ]; then
	mv "$UD/slot" "$DATA"
fi
mkdir -p "$DATA"

export SHELF_ROOT="$DATA"
export SHELF_ROMS="${ROMS_PATH:-$SD/Roms}"
export SHELF_HANDOFF="$NEXT"
# Where the emulator paks are, for the Emulator row to see which are installed.
export SHELF_EMU_DIRS="$SD/Emus/$PLAT:$SYS/paks/Emus"
# Where NextUI keeps each game's save-state slot, to show which can resume.
export SHARED_USERDATA_PATH="${SHARED_USERDATA_PATH:-$SD/.userdata/shared}"
# The Brick numbers its buttons its own way.
export SHELF_PAD=trimui
# The Brick's screen is 1024x768. On the Brick Shelf fills it; on a
# laptop running this pak as a preview, the window opens at that size.
export SHELF_WINDOW="${SHELF_WINDOW:-1024x768}"
# Only shelve the games named in the Top 100 collection lists, when the
# card has them. SHELF_ONLY set before the pak starts picks other lists.
if [ -z "$SHELF_ONLY" ]; then
	for only in "$SD/Collections/Top 100" "$SD/Roms/_Collections/Top 100" \
		"$SD/_Collections/Top 100"; do
		if [ -d "$only" ]; then
			export SHELF_ONLY="$only"
			break
		fi
	done
fi
[ -f "$DATA/trace-input" ] && export SHELF_TRACE_INPUT=1

section() {
	echo
	echo "== $*"
}

show() {
	for f in "$@"; do
		[ -e "$f" ] || continue
		if [ -d "$f" ]; then
			echo "$f/"
			ls -la "$f" 2>&1
		else
			echo "$f: $(tr '\n' ' ' < "$f" 2>/dev/null)"
		fi
	done
}

probe() {
	section "system"
	date
	uname -a
	show /etc/version /etc/os-release /etc/openwrt_release
	grep -m1 -i hardware /proc/cpuinfo
	head -3 /proc/meminfo
	ls /lib/libc.so* /lib/libc-* /lib/ld-linux* 2>&1
	for t in timeout mountpoint hwclock amixer aplay wpa_supplicant; do
		echo "$t: $(command -v "$t" || echo missing)"
	done

	section "display"
	show /sys/class/graphics/fb0/modes /sys/class/graphics/fb0/mode \
		/sys/class/graphics/fb0/virtual_size /sys/class/graphics/fb0/bits_per_pixel \
		/sys/class/graphics/fb0/stride
	ls -la /dev/fb* /dev/dri /dev/pvr* /dev/mali* /dev/disp 2>&1
	ls -la /usr/lib/libEGL* /usr/lib/libGLES* /usr/lib/libIMG* /usr/lib/libsrv* \
		/usr/lib/libpvr* /usr/lib/libmali* /usr/lib/libSDL2* 2>&1
	[ -d /sys/kernel/debug/dispdbg ] && echo "dispdbg present"

	section "input"
	cat /proc/bus/input/devices
	for d in /sys/class/input/event*; do
		echo "$(basename "$d") name=$(cat "$d/device/name" 2>/dev/null)" \
			"key=[$(cat "$d/device/capabilities/key" 2>/dev/null)]" \
			"abs=[$(cat "$d/device/capabilities/abs" 2>/dev/null)]" \
			"ff=[$(cat "$d/device/capabilities/ff" 2>/dev/null)]"
	done
	for d in /sys/class/input/event*; do
		for a in /sys/class/input/"$(basename "$d")"/device/abs*; do
			[ -e "$a" ] && echo "$a"
		done
	done

	section "power"
	for p in /sys/class/power_supply/*; do
		echo "$p"
		cat "$p/uevent" 2>&1
	done
	show /sys/class/backlight /sys/class/leds /sys/class/led_anim /sys/class/rtc /sys/class/udc
	for b in /sys/class/backlight/*; do
		show "$b/brightness" "$b/max_brightness"
	done

	section "audio"
	cat /proc/asound/cards 2>&1
	aplay -l 2>&1
	amixer scontrols 2>&1
	amixer contents 2>&1 | head -80

	section "storage"
	cat /proc/mounts
	df -h "$PAK" 2>&1
}

rom_tag() {
	# The system folder is the one directly under Roms, even for a game in
	# a subfolder of it such as "00) All but the Best (Gameboy)".
	rel="${1#"$SHELF_ROMS"/}"
	echo "${rel%%/*}" | sed -n 's/.*(\([^()]*\))$/\1/p'
}

# The pak with this tag, among the ones on the card and NextUI's own.
find_pak() {
	for pak in "$SD/Emus/$PLAT/$1.pak" "$SYS/paks/Emus/$1.pak"; do
		if [ -f "$pak/launch.sh" ]; then
			echo "$pak"
			return 0
		fi
	done
	return 1
}

# The emulator pak for a game: the one chosen for it in Shelf's Start menu
# (Config/emus.txt, "<tag><TAB><rom path>"), if that pak is installed,
# otherwise the one its Roms folder is tagged for.
emu_pak() {
	chosen="$(awk -F '\t' -v rom="$1" '$2 == rom { tag = $1 } END { print tag }' \
		"$DATA/Config/emus.txt" 2>/dev/null)"
	if [ -n "$chosen" ] && find_pak "$chosen"; then
		return 0
	fi
	tag="$(rom_tag "$1")"
	[ -n "$tag" ] || return 1
	find_pak "$tag"
}

# NextUI's Resume: its emulator loads the save state whose slot number is
# in the resume file. Shelf asks for it with a second handoff line,
# "resume"; NextUI keeps the game's last slot in
# .minui/<tag>/<rom file name>.txt. Anything else clears a stale request.
resume_slot() {
	RESUME="${SHELF_RESUME_SLOT:-/tmp/resume_slot.txt}"
	rm -f "$RESUME"
	# NextUI's quicksave from powering off in a game.
	if [ "$2" = auto ]; then
		echo 9 > "$RESUME"
		return 0
	fi
	[ "$2" = resume ] || return 0
	slot="${SHARED_USERDATA_PATH:-$SD/.userdata/shared}/.minui/$(rom_tag "$1")/$(basename "$1").txt"
	[ -s "$slot" ] && cp "$slot" "$RESUME"
	return 0
}

# NextUI's game timer, started and stopped as NextUI's menu does. Its sleep
# pauses the timer, so the log it keeps leaves sleep out, and Shelf reads
# each game's time back from it.
gametime() {
	gt="$SYS/bin/gametimectl.elf"
	[ -x "$gt" ] || gt="$(command -v gametimectl.elf)" || return 0
	"$gt" "$@" > /dev/null 2>&1
}

hooks() {
	[ -x "$SYS/bin/run_hooks.sh" ] && "$SYS/bin/run_hooks.sh" "$1"
}

# Powering off or restarting in a game: NextUI's emulator makes its
# quicksave, leaves one of these and exits, for NextUI to turn the Brick
# off. Shelf does not come back up first; the boot hook resumes the game
# when the Brick is turned on.
powering_off() {
	[ -f /tmp/poweroff ] || [ -f /tmp/reboot ]
}

# Run the chosen game through its emulator pak, with NextUI's hooks and
# game timer around it as NextUI's menu has.
play() {
	if ! pak="$(emu_pak "$1")"; then
		echo "$(date '+%F %T') no emulator pak for $1" >> "$DATA/shelf.log"
		return 0
	fi
	resume_slot "$1" "$2"
	export HOOK_CMD="'$pak/launch.sh' '$1'" HOOK_EMU_PATH="$pak/launch.sh"
	export HOOK_TYPE=rom HOOK_ROM_PATH="$1" HOOK_LAST="$HOOK_CMD"
	hooks pre-launch.d
	sh "$SYS/bin/governor.sh" performance 2>/dev/null
	gametime start "$1"
	"$pak/launch.sh" "$1"
	gametime stop_all
	hooks post-launch.d
}

# NextUI resumes the game the Brick was powered off in when it is turned
# back on, before its menu shows. This boot hook steps in first: it notes
# when the Brick went off, so the time it was off is not counted as play,
# and when Shelf handed the game out, it resumes it through Shelf, so
# quitting it comes back to the shelf.
BOOT_HOOK="$UD/.hooks/boot.d/shelf-resume.sync.sh"
boot_hook() {
	cat <<HOOK
#!/bin/sh
# Written by Shelf ($PAK) each time it starts.
SD="\${SDCARD_PATH:-/mnt/SDCARD}"
NOTE="\${SHARED_USERDATA_PATH:-\$SD/.userdata/shared}/.minui/auto_resume.txt"
CONF="$DATA/Config"
[ -f "\$NOTE" ] || exit 0
rom="\$SD\$(head -n 1 "\$NOTE")"
mkdir -p "\$CONF"
# When it went off: when NextUI wrote its note, at the power-off.
at="\$(stat -c %Y "\$NOTE" 2>/dev/null || date -r "\$NOTE" +%s 2>/dev/null || date +%s)"
line="\$(printf '%s\t%s' "\$at" "\$rom")"
offs="\$CONF/poweroffs.txt"
if [ "\$(tail -n 1 "\$offs" 2>/dev/null)" != "\$line" ]; then
	{ tail -n 49 "\$offs" 2>/dev/null; echo "\$line"; } > "\$offs.new" && mv -f "\$offs.new" "\$offs"
fi
[ -f "$PAK/launch.sh" ] && [ -f "\$CONF/playing.txt" ] || exit 0
[ "\$(head -n 1 "\$CONF/playing.txt" | cut -f 4-)" = "\$rom" ] || exit 0
killall -9 show2.elf > /dev/null 2>&1
SHELF_RESUME="\$rom" sh "$PAK/launch.sh"
if [ -f /tmp/poweroff ]; then
	poweroff_next || poweroff
elif [ -f /tmp/reboot ]; then
	reboot_next || reboot
fi
exit 0
HOOK
}
if [ -d "$UD" ] && [ "$(boot_hook)" != "$(cat "$BOOT_HOOK" 2>/dev/null)" ]; then
	mkdir -p "$(dirname "$BOOT_HOOK")"
	boot_hook > "$BOOT_HOOK"
	chmod +x "$BOOT_HOOK" 2>/dev/null
fi

# With "Open at Power-On" on, Shelf starts ahead of NextUI's menu: a hook
# that runs after the resume one above (so only when it did not already
# run Shelf, which leaves RAN behind). Quitting Shelf, Select or Menu held
# as it starts, or two starts in a row that never drew a frame (Shelf
# clears boot-tries once it has), go on to NextUI's menu instead.
RAN="${SHELF_RAN:-/tmp/shelf-ran}"
START_HOOK="$UD/.hooks/boot.d/shelf-start.sync.sh"
start_hook() {
	cat <<HOOK
#!/bin/sh
# Written by Shelf ($PAK) each time it starts.
CONF="$DATA/Config"
[ -f "$RAN" ] && exit 0
grep -qx 'boot=1' "\$CONF/shelf.state" 2>/dev/null || exit 0
[ -f "$PAK/launch.sh" ] || exit 0
# NextUI left a game to resume that Shelf did not hand out: let it.
[ -f "\${SHARED_USERDATA_PATH:-\${SDCARD_PATH:-/mnt/SDCARD}/.userdata/shared}/.minui/auto_resume.txt" ] && exit 0
tries="\$(cat "\$CONF/boot-tries" 2>/dev/null)"
case "\$tries" in '' | *[!0-9]*) tries=0 ;; esac
[ "\$tries" -lt 2 ] || exit 0
mkdir -p "\$CONF"
echo "\$((tries + 1))" > "\$CONF/boot-tries"
killall -9 show2.elf > /dev/null 2>&1
SHELF_BOOT=1 sh "$PAK/launch.sh"
if [ -f /tmp/poweroff ]; then
	poweroff_next || poweroff
elif [ -f /tmp/reboot ]; then
	reboot_next || reboot
fi
exit 0
HOOK
}
if [ -d "$UD" ] && [ "$(start_hook)" != "$(cat "$START_HOOK" 2>/dev/null)" ]; then
	mkdir -p "$(dirname "$START_HOOK")"
	start_hook > "$START_HOOK"
	chmod +x "$START_HOOK" 2>/dev/null
fi
: > "$RAN" 2>/dev/null

# Called from the boot hook: resume the game Shelf handed out before the
# power-off, then go on to the shelf.
if [ -n "$SHELF_RESUME" ]; then
	rm -f "$SHARED_USERDATA_PATH/.minui/auto_resume.txt"
	# Keep when it went out; the old clock reading means nothing after a
	# power-off, so NextUI's timer alone gives its time.
	playing="$DATA/Config/playing.txt"
	wall="$(head -n 1 "$playing" | cut -f 2)"
	boot="$(cat /proc/sys/kernel/random/boot_id 2>/dev/null)"
	printf '%s\t%s\t%s\t%s\n' - "$wall" "${boot:--}" "$SHELF_RESUME" > "$playing"
	play "$SHELF_RESUME" auto
	if powering_off; then
		sync
		exit 0
	fi
fi

if [ ! -f "$LOG" ]; then
	{
		echo "shelf brick probe, pak at $PAK"
		probe
	} > "$LOG" 2>&1
fi

while :; do
	rm -f "$NEXT"
	[ -f "$DATA/shelf.log" ] && mv -f "$DATA/shelf.log" "$DATA/shelf.log.1"
	# A game leaves the CPU held at full speed; the picker lets it scale
	# down when idle, as NextUI's menu does.
	sh "$SYS/bin/governor.sh" auto 2>/dev/null
	"$RUNNER" "$PAK/shelf" > "$DATA/shelf.log" 2>&1
	[ -f "$NEXT" ] || break
	rom="$(head -n 1 "$NEXT")"
	want="$(sed -n 2p "$NEXT")"
	rm -f "$NEXT"
	case "$rom" in
	'!poweroff')
		touch /tmp/poweroff
		break
		;;
	'!reboot')
		touch /tmp/reboot
		break
		;;
	esac
	play "$rom" "$want"
	powering_off && break
done
sync
