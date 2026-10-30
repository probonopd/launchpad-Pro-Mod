# Custom Mode data from Novation Components: what is known

Goal: let the Launchpad X load the Custom Modes made in Novation Components. Status:
**not implemented**; this page records what is known so far and what is missing.

## Source of the information

An open-source command-line tool, `ncc` (version 0.1.4, AGPL-3.0-or-later, by
taylordotfish), compiles a text description of a Custom Mode into a SysEx message
for the Launchpad X and Mini MK3 and was tested by its author on a Launchpad X
with firmware 2.0.1. The notes below are a reading of its source by a helper
agent, not our own capture of Components' traffic. Because the tool is AGPL, none
of its code is used here; only the byte layout (a file format) is described, and
any implementation has to be written independently.

## The message ncc writes (single SysEx, no checksum)

```
F0 00 20 29 02 <device> 20 00 45 40 7F 20 <name: length byte, ASCII, up to 16>
21 01 00
<64 pad records> <8 fader records> <trailer> F7
```

`<device>` is 0x0C for the Launchpad X, 0x0D for the Mini MK3. The Launchpad Pro
Mk3 is not supported by ncc. All data bytes are 0 to 127; there is no nibble or
7-bit packing and no length field apart from the name and the record lengths.

- **Empty record:** `40 <address>`
- **Defined record:** `<0x45 + L> <address> <type> <color> 00 <opt1> <opt2> <payload, L bytes>`
- Addresses: pads 0 to 63, faders 64 to 71.
- Types and payloads: note `01` (`00 <velocity> <note>`), CC `02`
  (`00 <cc> <on> <off>`), program change `03` (`00 00 <p> <p>`), key press `0C`.
- `opt1`/`opt2` bits: fixed channel (channel - 1 in `opt1`), global channel
  (`opt2` 0x40), toggle (`opt1` 0x20), aftertouch (`opt1` 0x10), fixed velocity
  (`opt2` 0x04), fader unipolar/bipolar (`opt2` 0x08 / 0x09).
- A trailer of key/value pairs follows the faders (active color, transposition
  and fader orientation).

## What is not known

- **Nothing about the exchange itself.** ncc only writes; it has no read-back, no
  slot selection (the user picks the slot on the device first) and no
  acknowledgement parsing. Components also reads modes, saves them as `.syx`, and
  talks to the device with other messages before it uploads. Without a capture of
  that traffic we cannot make Components accept our firmware as a Launchpad X it
  can write to.
- The meaning of several fixed bytes, and whether the Components `.syx` export is
  identical to ncc's message.
- Where the pad order of the 64 records maps to the physical grid.
- The master channel handling of the device.

## What would settle it

1. A `.syx` file saved from Novation Components with a test mode that uses each
   control type (a note pad, a CC pad, a toggle, a program change, a fader).
2. A recording of what Components sends and receives while it connects to a
   stock Launchpad X and uploads that mode (a MIDI monitor on the port, with the
   stock firmware installed).

With those, a parser for the message into the existing `ModeDef` structure, a
storage window for the eight slots in the external flash, and the Components
handshake can be written and tested against real data, instead of from the third-
party description above.

## Where this would be stored

Eight modes of about 700 bytes each fit into one 4 KB sector of the external flash,
inside a window with the same bounds rules as for any other saved data (see
[launchpad-x-technical-reference.md](launchpad-x-technical-reference.md), section 8).
