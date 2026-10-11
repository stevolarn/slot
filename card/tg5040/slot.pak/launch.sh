#!/bin/sh
# slot as a game picker for NextUI on the TrimUI Brick. slot shows its
# shelf, writes the chosen rom to $NEXT and exits; this loop runs that rom
# through NextUI's own emulator pak, the way NextUI's menu does, then
# starts slot again. Quitting slot without a game drops back to NextUI.
PAK="${SLOT_PAK:-$(cd "$(dirname "$0")" && pwd)}"
SD="${SDCARD_PATH:-/mnt/SDCARD}"
PLAT="${PLATFORM:-tg5040}"
SYS="${SYSTEM_PATH:-$SD/.system/$PLAT}"
DATA="${USERDATA_PATH:-$SD/.userdata/$PLAT}/slot"
NEXT="${SLOT_NEXT:-/tmp/slot-next}"
RUNNER="${AGS_SLOT_RUN:-/lib/ld-linux-aarch64.so.1}"
LOG="$DATA/probe.log"
mkdir -p "$DATA"

export SLOT_ROOT="$DATA"
export SLOT_ROMS="${ROMS_PATH:-$SD/Roms}"
export SLOT_HANDOFF="$NEXT"
# Only shelve the games named in the Top 100 collection lists, when the
# card has them. SLOT_ONLY set before the pak starts picks other lists.
if [ -z "$SLOT_ONLY" ]; then
	for only in "$SD/Collections/Top 100" "$SD/Roms/_Collections/Top 100" \
		"$SD/_Collections/Top 100"; do
		if [ -d "$only" ]; then
			export SLOT_ONLY="$only"
			break
		fi
	done
fi
[ -f "$DATA/trace-input" ] && export SLOT_TRACE_INPUT=1

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

	section "rumble"
	ls /sys/class/gpio 2>&1

	section "storage"
	cat /proc/mounts
	df -h "$PAK" 2>&1
}

emu_pak() {
	tag="$(basename "$(dirname "$1")" | sed -n 's/.*(\([^()]*\))$/\1/p')"
	[ -n "$tag" ] || return 1
	for pak in "$SD/Emus/$PLAT/$tag.pak" "$SYS/paks/Emus/$tag.pak"; do
		if [ -f "$pak/launch.sh" ]; then
			echo "$pak"
			return 0
		fi
	done
	return 1
}

hooks() {
	[ -x "$SYS/bin/run_hooks.sh" ] && "$SYS/bin/run_hooks.sh" "$1"
}

if [ ! -f "$LOG" ]; then
	{
		echo "slot brick probe, pak at $PAK"
		probe
	} > "$LOG" 2>&1
fi

while :; do
	rm -f "$NEXT"
	[ -f "$DATA/slot.log" ] && mv -f "$DATA/slot.log" "$DATA/slot.log.1"
	"$RUNNER" "$PAK/slot" > "$DATA/slot.log" 2>&1
	[ -f "$NEXT" ] || break
	rom="$(head -n 1 "$NEXT")"
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
		echo "$(date '+%F %T') no emulator pak for $rom" >> "$DATA/slot.log"
		continue
	fi
	export HOOK_CMD="'$pak/launch.sh' '$rom'" HOOK_EMU_PATH="$pak/launch.sh"
	export HOOK_TYPE=rom HOOK_ROM_PATH="$rom" HOOK_LAST="$HOOK_CMD"
	hooks pre-launch.d
	sh "$SYS/bin/governor.sh" performance 2>/dev/null
	"$pak/launch.sh" "$rom"
	hooks post-launch.d
done
sync
