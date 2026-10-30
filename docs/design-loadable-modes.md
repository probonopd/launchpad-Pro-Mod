# Design idea: modes loaded from the external flash (not implemented)

Status: **idea only**, nothing of this exists in the firmware. Written down on
2026-10-30 so that the reasoning is not lost. Facts about the hardware are in
[launchpad-x-technical-reference.md](launchpad-x-technical-reference.md).

## Question

Could additional modes be stored as executable code on the external flash chip and
loaded when needed, instead of being compiled into the 80 KB application area?

## Answer: yes, as RAM overlays

**Why RAM.** The external flash sits on a plain SPI bus (SPI2), not on a
memory-mapped quad-SPI interface, so the processor cannot execute code from it
directly. The alternative, copying code into the internal flash at runtime,
would mean erasing and programming internal sectors from the running program,
right next to the bootloader's sectors and the validity record: exactly the risk
this project avoids. The Cortex-M4 can execute code placed in RAM, so a module is
read from the external flash into a reserved RAM block and called from there.

**Budget.**
- RAM: 64 KB in total. The current firmware uses about 29.6 KB of static data
  (Note Mode build, project saving left out), plus the stack. About 25 to 30 KB
  would be available for one overlay, to be measured when it is built.
- Only one overlay can be resident at a time. Switching mode copies the next one
  in; reading 20 KB over SPI takes a few milliseconds.
- External flash: the 760 KB erased area (see the reference, section 8) could hold
  many modules; a window would be reserved for them, with the same bounds rules as
  for saved data.

## What it would need

1. **A reserved RAM region** in the linker script for the overlay (code, data and
   its own working memory), taken from the main firmware's RAM. Simplest variant:
   every module is linked to run at that one fixed address. More flexible:
   position-independent code with a small relocation step in the loader.
2. **A stable interface** between the core and the modules: a table of function
   pointers the core hands to the module (set LED, send MIDI, read time, request
   mode switch, ...) and entry points the module provides (enter, exit, pad
   event, button event, tick, draw). This table is versioned.
3. **A module header** with magic, interface version, load address, sizes, entry
   point offset and a CRC32 (a signature if modules should be restricted to
   trusted authors). The loader refuses anything that does not verify; a refused
   module leaves the previous mode running. No fallback guessing.
4. **An upload path** that writes modules into the reserved flash window, for
   example a SysEx command handled like the diagnostic's, with the same bounds
   checks and sector-only erases as for projects.
5. **A build flow** that produces module binaries (separate `no_std` crates,
   linked with a module linker script, checked by a verifier like
   `tools/verify_lpx.py`) and host tests for the loader logic.

## Advantages

- Modes beyond the 80 KB application limit.
- Updating or adding a mode writes only to our own window in the external flash:
  the internal flash, the validity record and the bootloader staging area are not
  touched. Only the loader itself needs a normal firmware update, once.

## Risks and costs

- No memory protection: a faulty module can hang or crash the firmware. A
  watchdog and the bootloader's key combination (Capture MIDI at plug-in) remain
  the way out; this is a crash risk, not a brick risk, as long as the loader never
  writes to the internal flash.
- Code that runs from user-writable storage runs whatever is written there. The
  CRC (or signature) check and the write window are the mitigation.
- Complexity: interface versioning, module build and debugging, relocations if
  position independence is chosen.
- Applies to CoreFW only. The Novation firmware has no such mechanism.

## Cheaper alternative first

The size-optimized build leaves about 23 KB in the application area. A few more
modes fit there as normal code. Overlays pay off only after that space is used.

## Open points to measure before deciding

- Real free RAM after the stack and the working memory of the existing modes.
- Whether the core can safely hand a function table to RAM code under the
  existing task structure (interrupt handlers must never call into an overlay).
- SPI read speed in the firmware (affects mode-switch latency).
