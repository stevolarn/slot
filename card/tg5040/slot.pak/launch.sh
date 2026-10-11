#!/bin/sh
# Brick probe: runs from NextUI's Tools as a pak. Writes what the Brick
# offers to probe.log, then runs slot with the input trace on and stops
# it after SLOT_PROBE_S seconds so NextUI comes back.
PAK="${SLOT_PAK:-$(cd "$(dirname "$0")" && pwd)}"
SECS="${SLOT_PROBE_S:-90}"
LOG="$PAK/probe.log"
export SLOT_ROOT="$PAK"
export SLOT_TRACE_INPUT=1

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

{
	echo "slot brick probe, pak at $PAK"
	probe
} > "$LOG" 2>&1

[ -f "$PAK/slot.log" ] && mv -f "$PAK/slot.log" "$PAK/slot.log.1"
[ -f "$PAK/input-trace.log" ] && mv -f "$PAK/input-trace.log" "$PAK/input-trace.log.1"

chmod +x "$PAK/System/slot" 2>/dev/null
/lib/ld-linux-aarch64.so.1 "$PAK/System/slot" > "$PAK/slot.log" 2>&1 &
pid=$!
echo "slot pid $pid, stopping after ${SECS}s" >> "$LOG"

(
	sleep "$SECS"
	kill "$pid" 2>/dev/null
	sleep 2
	kill -9 "$pid" 2>/dev/null
) &
watch=$!

wait "$pid"
echo "slot exited with $?" >> "$LOG"
kill "$watch" 2>/dev/null
sync
