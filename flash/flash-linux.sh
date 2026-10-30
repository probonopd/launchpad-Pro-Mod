#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 launchpad-Pro-Mod contributors
#
# Flashes a Launchpad X firmware (.syx) through the bootloader on Linux, using amidi.
# This is the method that was used on the one Launchpad X we tested on.
# It refuses anything that is not a Launchpad X image or a Launchpad X in its bootloader.
#
# Usage: flash-linux.sh [--yes] FIRMWARE.syx
# Before: unplug the Launchpad, hold "Capture MIDI" (top row, rightmost) and plug it in.
#         The pads show the bootloader; no sound is made, no mode is active.
set -eu

yes=no
if [ "${1:-}" = "--yes" ]; then
    yes=yes
    shift
fi
fw=${1:-}
[ -n "$fw" ] && [ -f "$fw" ] || { echo "usage: $0 [--yes] FIRMWARE.syx" >&2; exit 2; }
command -v amidi >/dev/null || { echo "amidi not found (package alsa-utils)" >&2; exit 2; }

# 1. The file must be a Launchpad X image: SysEx header F0 00 20 29 00 71 02 0C (product 0x0C).
head8=$(head -c 8 "$fw" | od -An -tx1 | tr -d ' \n')
[ "$head8" = "f00020290071020c" ] || { echo "not a Launchpad X firmware file: $fw" >&2; exit 1; }

# 2. If a SHA256SUMS file lies next to it, the image must match.
dir=$(dirname "$fw")
if [ -f "$dir/SHA256SUMS" ]; then
    (cd "$dir" && grep " $(basename "$fw")\$" SHA256SUMS | sha256sum -c -) \
        || { echo "checksum mismatch, not flashing" >&2; exit 1; }
else
    echo "warning: no SHA256SUMS next to the file, checksum not verified" >&2
fi
echo "image sha256: $(sha256sum "$fw" | cut -d' ' -f1)"

# 3. The Launchpad must be in the bootloader ("LPX BL M"), not running the firmware.
port=$(amidi -l | awk '/Bootloader|BL M/ { print $2; exit }')
if [ -z "$port" ]; then
    echo "No Launchpad X in bootloader mode found." >&2
    echo "Unplug it, hold Capture MIDI, plug it in while holding, then run this again." >&2
    exit 1
fi

# 4. The bootloader must identify itself as a Launchpad X one (family 03 11).
reply=$(amidi -p "$port" -S 'F07E7F0601F7' -d -t 2 2>/dev/null | tr -d '\n' | sed 's/  */ /g')
case "$reply" in
    *"F0 7E 00 06 02 00 20 29 03 11"*) echo "bootloader on $port: $reply" ;;
    *) echo "unexpected answer from $port: '$reply', not flashing" >&2; exit 1 ;;
esac

if [ "$yes" != yes ]; then
    printf 'Flash %s to the Launchpad X on %s? Do not unplug it. [y/N] ' "$(basename "$fw")" "$port"
    read -r answer
    [ "$answer" = y ] || [ "$answer" = Y ] || { echo "cancelled"; exit 1; }
fi

# 5. Send. About 30 seconds. A failed transfer leaves the old firmware in place; the
#    bootloader stays available, so simply try again.
amidi -p "$port" -i 10 -s "$fw"
echo "sent; waiting for the Launchpad to restart ..."
sleep 6
if amidi -l | grep -q 'Launchpad X LPX'; then
    echo "done: the Launchpad X is running the new firmware"
else
    echo "The Launchpad did not come back as the firmware (red F on the pads = transfer failed)." >&2
    echo "It is still in the bootloader or restarting; unplug, hold Capture MIDI, plug in and try again." >&2
    exit 1
fi
