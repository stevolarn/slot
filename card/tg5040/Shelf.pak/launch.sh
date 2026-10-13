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

emu_pak() {
	tag="$(rom_tag "$1")"
	[ -n "$tag" ] || return 1
	for pak in "$SD/Emus/$PLAT/$tag.pak" "$SYS/paks/Emus/$tag.pak"; do
		if [ -f "$pak/launch.sh" ]; then
			echo "$pak"
			return 0
		fi
	done
	return 1
}

# NextUI's Resume: its emulator loads the save state whose slot number is
# in the resume file. Shelf asks for it with a second handoff line,
# "resume"; NextUI keeps the game's last slot in
# .minui/<tag>/<rom file name>.txt. Anything else clears a stale request.
resume_slot() {
	RESUME="${SHELF_RESUME_SLOT:-/tmp/resume_slot.txt}"
	rm -f "$RESUME"
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
	if ! pak="$(emu_pak "$rom")"; then
		echo "$(date '+%F %T') no emulator pak for $rom" >> "$DATA/shelf.log"
		continue
	fi
	resume_slot "$rom" "$want"
	export HOOK_CMD="'$pak/launch.sh' '$rom'" HOOK_EMU_PATH="$pak/launch.sh"
	export HOOK_TYPE=rom HOOK_ROM_PATH="$rom" HOOK_LAST="$HOOK_CMD"
	hooks pre-launch.d
	sh "$SYS/bin/governor.sh" performance 2>/dev/null
	gametime start "$rom"
	"$pak/launch.sh" "$rom"
	gametime stop_all
	hooks post-launch.d
done
sync
