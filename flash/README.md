# Flashing the Launchpad X

**Launchpad X only.** These scripts refuse images for other models and anything that is not
a Launchpad X bootloader. The firmware was tested on one Launchpad X and on no other model.

1. Unplug the Launchpad X. Hold **Capture MIDI** (top row, rightmost button) and plug it
   in while holding. It is now in its bootloader and cannot be damaged by what follows.
2. Run the script with the firmware file next to `SHA256SUMS`:

   - Linux (tested): `./flash-linux.sh launchpad-x-novation-sequencer.syx`
     (needs `amidi` from alsa-utils)
   - Any system (NOT TESTED by us): `python3 flash.py launchpad-x-novation-sequencer.syx`
     (needs `pip install mido python-rtmidi`)

3. Wait about 30 seconds. The Launchpad restarts into the new firmware. A red "F" on the
   pads means the transfer failed; nothing was changed, repeat from step 1.

Two variants: `launchpad-x-launchpad95-sequencer.syx` (Launchpad95-style sequencer) and
`launchpad-x-novation-sequencer.syx` (Novation-style). To go back to the original firmware use
Novation Components (see docs/user-manual.md, chapter 12).
