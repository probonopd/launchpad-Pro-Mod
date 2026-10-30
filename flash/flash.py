#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 launchpad-Pro-Mod contributors
"""Flashes a Launchpad X firmware (.syx) through the bootloader on Linux, macOS or Windows.

NOT TESTED BY US. We only ran the Linux amidi script (flash-linux.sh) on real hardware.
This script does the same checks and sends the same messages with the Python packages
mido and python-rtmidi (pip install mido python-rtmidi). Prefer flash-linux.sh on Linux.

Usage: flash.py [--yes] FIRMWARE.syx
Before: unplug the Launchpad, hold "Capture MIDI" (top row, rightmost), plug it in.
"""
import hashlib
import os
import sys
import time

try:
    import mido
except ImportError:
    sys.exit("needs mido and python-rtmidi: pip install mido python-rtmidi")

HEADER = bytes.fromhex("f00020290071020c")  # SysEx header of Launchpad X images (product 0x0C)
INQUIRY_REPLY_START = [0x7E, 0x00, 0x06, 0x02, 0x00, 0x20, 0x29, 0x03, 0x11]


def split_messages(data):
    messages, start = [], None
    for i, byte in enumerate(data):
        if byte == 0xF0:
            start = i
        elif byte == 0xF7 and start is not None:
            messages.append(list(data[start + 1:i]))
            start = None
    return messages


def find(names, words):
    for name in names:
        if any(w in name for w in words):
            return name
    return None


def main():
    args = [a for a in sys.argv[1:] if a != "--yes"]
    assume_yes = "--yes" in sys.argv[1:]
    if len(args) != 1 or not os.path.isfile(args[0]):
        sys.exit(__doc__)
    path = args[0]
    data = open(path, "rb").read()
    if not data.startswith(HEADER):
        sys.exit("not a Launchpad X firmware file: " + path)

    sums = os.path.join(os.path.dirname(os.path.abspath(path)), "SHA256SUMS")
    digest = hashlib.sha256(data).hexdigest()
    if os.path.isfile(sums):
        wanted = [l.split()[0] for l in open(sums) if l.split()[-1:] == [os.path.basename(path)]]
        if digest not in wanted:
            sys.exit("checksum mismatch, not flashing")
    else:
        print("warning: no SHA256SUMS next to the file, checksum not verified")
    print("image sha256:", digest)

    out_name = find(mido.get_output_names(), ("Bootloader", "BL M"))
    in_name = find(mido.get_input_names(), ("Bootloader", "BL M"))
    if not out_name or not in_name:
        sys.exit("No Launchpad X in bootloader mode found. Unplug it, hold Capture MIDI, plug it in and retry.")

    with mido.open_output(out_name) as out, mido.open_input(in_name) as inp:
        out.send(mido.Message("sysex", data=[0x7E, 0x7F, 0x06, 0x01]))
        reply, end = None, time.time() + 2
        while time.time() < end and reply is None:
            for msg in inp.iter_pending():
                if msg.type == "sysex":
                    reply = list(msg.data)
            time.sleep(0.05)
        if reply is None or reply[: len(INQUIRY_REPLY_START)] != INQUIRY_REPLY_START:
            sys.exit("unexpected answer from the bootloader: %r, not flashing" % (reply,))
        print("bootloader on", out_name)

        if not assume_yes and input("Flash %s? Do not unplug it. [y/N] " % os.path.basename(path)).lower() != "y":
            sys.exit("cancelled")

        messages = split_messages(data)
        for i, body in enumerate(messages):
            out.send(mido.Message("sysex", data=body))
            time.sleep(0.01)  # the bootloader needs a little time per message
            if i % 200 == 0:
                print("sent %d / %d" % (i, len(messages)))
    time.sleep(6)
    if find(mido.get_output_names(), ("Launchpad X LPX", "LPX")) and not find(mido.get_output_names(), ("Bootloader", "BL M")):
        print("done: the Launchpad X is running the new firmware")
    else:
        sys.exit("The Launchpad did not come back as the firmware (red F on the pads = transfer failed). Unplug, hold Capture MIDI, plug in and retry.")


main()
