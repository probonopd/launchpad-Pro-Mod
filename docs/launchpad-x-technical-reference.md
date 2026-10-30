# Novation Launchpad X: technical reference from our own measurements

Everything here was found on one real Launchpad X (shipped with firmware 2.0.1,
application build 422) during the Seq95 / Note Mode port, between 2026-10-29 and
2026-10-30. Each fact is marked:

- **[measured]** read from the device or from files we hold, reproducible
- **[derived]** a conclusion from measurements; the reasoning is given
- **[unknown]** not established

The chronological log with every command and result is
[lpx-port-safety.md](lpx-port-safety.md). The serial number and the chip's unique
id of the unit are deliberately left out of the documents.

## 1. Hardware

| Part | Fact |
| --- | --- |
| MCU | STM32F4-class Cortex-M4. The chip's size register (0x1FFF7A22) reads **128 KB of flash**; CoreFW builds with the `stm32f401re` support package, which assumes 512 KB. **[measured]** |
| Internal flash | `0x08000000`-`0x0801FFFF`. Sectors: 0-3 are 16 KB each, sector 4 is 64 KB, **sectors 5-7 do not exist**. **[measured/derived]** |
| RAM | 64 KB at `0x20000000`; the stock firmware starts the stack at `0x2000FFF8`, CoreFW at `0x20010000`. **[measured]** |
| Read protection | Option bytes `ef aa 10 55 ef aa 10 55 ff 7f 00 80 ff 7f 00 80`: byte 1 = `0xAA` = level 0, nothing is read-protected. The write-protect words are not fully interpreted. **[measured]** |
| External flash | Macronix, JEDEC id `C2 20 14`, 8 Mbit = **1 MB**, on SPI2. **[measured]** |
| OTP | 560 bytes read, all `FF`. **[measured]** |
| ST ROM bootloader | 30 KB read from `0x1FFF0000`; not studied. It only starts with the BOOT0 pin high at reset (not accessible without opening the case). **[measured/unknown]** |
| USB ids | Application `1235:0103` "Launchpad X"; bootloader `1235:1103` "Launchpad X Bootloader". **[measured]** |
| MIDI ports | Stock: "LPX DAW", "LPX MIDI". Bootloader: "LPX BL M". CoreFW: "LPX (DAW)", "LPX (MIDI)". **[measured]** |

## 2. Internal flash map

| Range | Content |
| --- | --- |
| `0x08000000`-`0x0800A0BF` | Novation bootloader, 40,128 bytes. Vector table: SP `0x2000FFF8`, reset `0x08009D1D`. Version text "000238" at `0x080001F8`. Strings: "Launchpad X Bootloader", "LPX Bootloader", "LPX BL MIDI", "Onboarding", "Launchpad Mini MK3" (the bootloader code base is shared with the Mini MK3). **[measured]** |
| `0x0800A0C0`-`0x0800BFFF` | 8,000 bytes of `FF` **inside the bootloader's sector 2**. Unusable: sector 2 cannot be erased without destroying the bootloader. **[derived]** |
| `0x0800C000`-`0x0801FFEF` | The application (sectors 3 and 4), 80 KB. The image starts with the vector table; an ASCII version string sits at offset `0x1F8` ("000351" in stock 351, "000999" in CoreFW; the same offset as in the bootloader). After the image the rest is `FF`. **[measured]** |
| `0x0801FFF0`-`0x0801FFFF` | **Validity record** written by the bootloader after a successful update: four little-endian words: `0`, version (`0x351`), image length, CRC32 of the image (polynomial `0x04C11DB7`, MSB first, initial value `0xFFFFFFFF`, no final xor; reproduced exactly). **[measured]** |

The bootloader is software in erasable flash, not hardware. **[measured]**
The only copy of it we know of is the one we dumped; keep it safe.

## 3. External flash map (1 MB)

| Range | Content |
| --- | --- |
| `0x000000`-`0x001FFF` | Identity block: "Focusrite Novation", "Launchpad Mini MK3", the unit's serial number. Never touch. **[measured]** |
| `0x002000`-`0x002FFF` | `FF`. **[measured]** |
| `0x003000`-`0x03FFFF` | **Bootloader staging area**: the last received update image is stored here byte for byte starting at `0x3000`; remnants of earlier, larger images and some zeros follow. **[measured]** |
| `0x040000`-`0x0FDFFF` | 760 KB, all `FF`. Whether Novation's own firmware ever uses it is **[unknown]** (see section 8). |
| `0x0FE000`-`0x0FFFFF` | CoreFW's settings window ("COREFW" header at `0x0FE000`). **[measured]** |

CoreFW's driver exposes only this last 8 KB as "flash" (offset 0 of the driver is
`0x0FE000` of the chip). The diagnostic reads the chip by physical offset.

## 4. How a firmware update works (bootloader)

1. Enter the bootloader: unplug, hold **Capture MIDI**, plug in. The device
   re-enumerates as `1235:1103`. **[measured]**
2. The host sends the update as SysEx on the bootloader's MIDI port. The files
   (`.syx`) contain: an init message (`0x71`, family `0x02`, product `0x0C` for the
   Launchpad X), a header message (`0x7C`: version digits, image size, CRC), many
   write messages (`0x72`, 32 bytes of image each, spread over nibbles), and a
   finish message (`0x73`). No addresses: the destination is decided by the
   bootloader. **[measured from syxtool and the files]**
3. The bootloader stores the received image in the external flash at `0x3000`
   and checks it against the CRC in the header. **[derived from the dumps]**
4. Only if the check passes, the image is copied into the internal flash
   (`0x0800C000` upward) and the validity record is written at the end of the
   flash. **[derived]**
5. A failed transfer ends with a blinking red **F** on the pads and the unit stays
   in the bootloader. We saw this after sending with a Linux browser; the
   installed application was not touched. **[measured/derived]**
6. The bootloader accepts a downgrade (build 422 to 351 worked). **[measured]**
7. A start pad (bottom right) in the bootloader starts the application again;
   the bootloader also has pads for the USB device id, brightness and the
   "Onboarding" mass-storage toggle (from Novation's Programmer's Reference).
   **[measured/manual]**

## 5. Identity and version messages

Universal Device Inquiry (`F0 7E 7F 06 01 F7`), reply `F0 7E 00 06 02 00 20 29 <family 2 bytes> 00 00 <4 version digits> F7`:

| State | Family | Version digits |
| --- | --- | --- |
| Stock application as shipped (2.0.1) | `03 01` | 0 4 2 2 |
| Stock application 351 | `03 01` | 0 3 5 1 |
| CoreFW (all our builds) | `03 01` | 0 9 9 9 |
| Bootloader | `03 11` | 0 2 3 8 |

Novation's Programmer's Reference prints `13 01` / `13 11` for the family bytes;
the device answers `03 01` / `03 11`. Trust the device.

## 6. Stock application 351 (offline analysis of the file)

- Keil-style C runtime; entry chain Reset, SystemInit (`0x0801C49C`), `__main`,
  `main` at `0x0800C378`, which initializes peripherals and the USB/MIDI engine
  and enters a super-loop at `0x0800C522`.
- Real interrupt handlers: SysTick (a tick counter), TIM1_UP, TIM2, OTG_FS (USB).
  About 50 other vectors are `b .` stubs.
- The application uses SPI2 (external flash) and SPI3. It never programs or
  erases the internal flash: the FLASH controller is touched only to set wait
  states and caches.
- Not yet traced: the button dispatch and the LED ownership, needed for an
  injection approach. **[unknown]**

## 7. CoreFW and our port

- Build the Launchpad X with `cargo lpx` only. It uses size optimization
  (`opt-level = "s"`, about 58 KB) because the speed-optimized build (80 KB)
  does not fit the 80 KB application area. A plain `cargo build` overflows and the
  linker refuses it, which is the safe outcome.
- The firmware modes and their buttons: the [User Manual](user-manual.md) (start
  here), [launchpad-x-modes.md](launchpad-x-modes.md), the sequencer
  [seq95-launchpad-x.md](seq95-launchpad-x.md), Note Mode with Chord
  [note-mode-launchpad-x.md](note-mode-launchpad-x.md) and the Custom Modes
  [custom-modes-launchpad-x.md](custom-modes-launchpad-x.md). The pictures are
  generated from the real code (`tools/framedump`, `tools/make_pictures.py`).
- Host crates with tests: `seq95` (28), `notemode` (22), `custommode` (14), `safestore` (12, including power cuts at every step of a save).
- Application size at the last check: about 68 KB of the 80 KB application area (about 12 KB free).
- None of our builds, nor upstream CoreFW, contains a flash unlock key or a
  reference to the internal flash controller: they cannot erase or program the
  internal flash, so they cannot overwrite the bootloader. **[measured]**

## 7a. RAM and stack budget

The chip has 64 KB of RAM (`0x20000000`-`0x2000FFFF`); the stack starts at the top. In the
current build the static data ends at `0x20007DF4` (32 KB), leaving 33,292 bytes for the
stack. `tools/verify_lpx.py` builds the call graph from the disassembly and requires a
margin of 4 KB over the worst case: at startup (the main task plus the functions that build
the big statics: 13,312 bytes) and while running (the main task plus the deepest app
handler plus 2 KB for interrupts: 23,256 bytes). Two earlier builds ran out of stack at
startup and did not start at all (see sections 31 and 31a of
[lpx-port-safety.md](lpx-port-safety.md)). Rules that follow: never build a big structure
by value on the stack at startup (build it in its own static, in a function of its own),
never add a large buffer to a structure that lives in the app host (put it on the stack for
the moment it is needed), prefer in-place construction of big arrays (`[const { X }; N]`),
and run the verifier on every build before sending it.

## 8. Using the free external flash (what can and cannot be claimed)

The question: can we store data (for example sequencer projects) in the erased
760 KB without risk of a brick?

**What is established**
- Writing the external flash can never change the internal flash, so it cannot
  directly damage the bootloader or the application. **[derived: separate chip,
  separate bus]**
- The bootloader's own data in the external flash is the identity block
  (`0x0000-0x1FFF`) and the staging area (`0x3000-0x3FFFF`). An area outside them
  that is erased now and is not CoreFW's settings is not used by the bootloader's
  update path. **[measured]**

**What is not established**
- Whether the bootloader depends on the external flash in other ways (for example
  reading the identity block at every start). We never write there.
- Whether Novation's own application stores data in `0x040000-0x0FDFFF`
  (for example Custom Modes uploaded with Components). Consequence: a collision of
  data between the two firmwares, not a brick.

**Conclusion.** Using a small block (the proposal is 56 KB at
`0x0F0000-0x0FDFFF`) carries a *very low* brick risk, not zero. The remaining risk
is a software error in our own writing code that erases or writes outside the
block, for example into the identity block or the staging area. That risk is
controlled by the design, not by the location:
1. one fixed window, with compile-time constants and every write and erase checked
   against it, refusing anything outside (no fallback);
2. sector erase only (never a chip or block erase), and no change of the chip's
   protection registers;
3. every record carries a signature and a CRC, and an unreadable record is treated
   as empty;
4. a host-side test of the bounds logic and a read-only verification dump
   (`tools/lpx_dump.py`) before and after the first real write;
5. the same flashing gates and recovery as before.
It is the "700 KB" that should not be used: only the chosen window is.

**What was built (2026-10-30).** The window is **`0x0DA000`-`0x0FDFFF`** (36 sectors of
4 KB = 144 KB, ending where CoreFW's settings begin), larger than the first proposal
because eight 7 KB projects each need two banks of two sectors. The design, the
record format, the power-cut tests and the three layers of bounds checks are in
[storage-design.md](storage-design.md). Compile-time checks fix the window above
`0x080000` and require it to end exactly at the settings window; the driver refuses
every write unless the chip answers with the expected Macronix id.

## 9. Tools we built

| Tool | Purpose |
| --- | --- |
| `cargo lpx` | Build the Launchpad X firmware (size-optimized) |
| `cargo lpxdiag` | Build the read-only flash diagnostic firmware |
| `tools/verify_lpx.py` | Check a build before sending it: segments inside the application area, vector table, product id, `.syx` decodes to the binary |
| `tools/lpx_dump.py` | Read the diagnostic's dumps over MIDI (checked chunks, sector maps) |
| `tools/verify_store.py` | Read-only check of the saved data in a dump, and a check that two dumps differ only in the allowed areas |
| `tools/diag_host_test/run.sh` | Test the diagnostic's protocol on the computer |
| `amidi -p <port> -i 10 -s file.syx` | Send an update on Linux (the browser tools fail here) |

## 10. Corrections of earlier assumptions

| Assumed | Found |
| --- | --- |
| 512 KB internal flash, free flash up to `0x08080000` | 128 KB; flash ends at `0x08020000` |
| The red F might mean a downgrade is refused | It was the Linux browser transfer (CoreFW issue 9); the downgrade is accepted |
| The application might use the flash beyond its image | It does not; there is none |
| Device family bytes `13 01` / `13 11` | `03 01` / `03 11` |
| The driver's "flash" is the whole chip | It is an 8 KB window at the end of the chip |
| Ghidra with the ARM profile can analyze the image | It decodes ARM instead of Thumb; objdump in Thumb mode was used |

## 11. Still unknown

- Whether the bootloader starts when the application image is broken (never
  tested on purpose; the staging and CRC design makes a bad image unlikely).
- What the write-protect option words mean exactly.
- Whether Novation's firmware uses `0x040000-0x0FDFFF` of the external flash.
- Button dispatch and LED ownership in the stock firmware (needed to keep
  Novation's modes next to Seq95).
- Pad response and LED timing of the size-optimized build on real pads.
