# Launchpad X: Seq95 port - findings and brick-safety protocol

> Scope: everything in this log concerns the Launchpad X, tested on one unit. Nothing was
> tested on other models. See [tested-hardware.md](tested-hardware.md).

Status when this log was started (2026-10-29): analysis and build only, nothing flashed.
Later sections record the flashes on one Launchpad X; the last section is the current state.
Date: 2026-10-29

## 1. Facts established (with source)

| Fact | Source |
|------|--------|
| MCU: STM32F401RE | core/Cargo.toml (`embassy-stm32/stm32f401re` for `launchpad-x`) |
| Bootloader region: 0x08000000 - 0x0800BFFF (48 KB) | core `launchpad-x/memory.x` (app starts at 0x0800C000) |
| App region used by CoreFW: 0x0800C000, length 0x14000 (80 KB) | `launchpad-x/memory.x` |
| Stock LPX 351 image: 68282 bytes, SP 0x2000FFF8, reset 0x0801C86D | LPX-351.bin header |
| CoreFW lpx (unmodified) built image ends at 0x0801A1F8 = 57,848 bytes | readelf LOAD segments of target/thumbv7em-none-eabihf/release/core |
| Headroom for Seq95 inside 80 KB: about 24 KB | 81,920 - 57,848 |
| External SPI flash: 1 MB on SPI2 (PB12-15); settings in last 8 KB | `common/storage/external.rs` (non-Pro-Mk3 constants) |
| Bootloader entry: unplug, hold Capture MIDI, plug in USB | CoreFW readme, Novation docs |
| Bootloader shows a MIDI port "Launchpad MIDI Bootloader"; update = SysEx blocks, CRC32 | tools/flash.py, tools/syxtool.py |
| Novation says the bootloader itself is never updated by .syx files | Novation manuals (Launchpad S/Mini) |

## 2. NOT verified (must not be assumed)

1. That the bootloader checks the Capture MIDI key BEFORE validating or jumping
   to the application image. This is the single assumption behind "always
   recoverable". The bootloader is not in any .syx/.bin we have; it can only be
   verified by (a) a real test with the device, or (b) a flash dump via SWD.
2. That the bootloader refuses images larger than 80 KB or outside 0x0800C000+.
   Never rely on this; our linker script must keep us inside it ourselves.
3. That the bootloader's flash-erase step cannot touch 0x08000000-0x0800BFFF
   for a crafted/corrupt SysEx stream. We only send syxtool-generated files
   whose blocks we verify (section 4), so this is not exercised.
4. Whether the STM32 option bytes (RDP/write protection) lock the bootloader
   sectors. Unknown. Treat the bootloader as unprotected and unrecoverable if
   damaged (no SWD recovery assumed).

## 3. Why a bad application image should not brick (reasoning, not proof)

- The .syx update only writes the application area, per the stock protocol.
- Community use: CoreFW and lpp-performance-cfw are flashed by many users and
  the standard recovery is "enter bootloader, flash stock via Components".
  Community-reported, not independently verified by us.
- Failure modes that DO brick (we must avoid): writing below 0x0800C000;
  a bootloader that interprets a bad image as a reason to skip bootloader mode.

## 4. Protocol - every step, in order

Gate 0 (before touching hardware, all must pass):
- [ ] Keep known-good files: `src/utility/firmware/stock/LPX-351.syx`
      sha256 a30c68166bca0ba4e586553b835f90bfeaeb3691a1707c2e2388462ee144e2d4
      (.bin sha256 d2792fe6219af27f53572eb479ab192c0c0eedc6822c442bab0962c7c56a9fc1).
      Copy to a second medium. Also confirm Novation Components can reach the
      device (the official rollback path, needs a browser with WebMIDI).
- [ ] Our build's linker script origin is 0x0800C000 and length <= 0x14000.
      Unchanged from CoreFW `launchpad-x/memory.x`. Any edit to memory.x is a STOP.
- [ ] Built ELF: all LOAD segments inside 0x0800C000-0x08020000 (readelf -lW).
- [ ] Final image size < 80 KB with at least 2 KB margin.
- [ ] Parse the generated .syx with our own script: header product ID = 0x0C
      (LPX), declared size = image size, every block's address is within
      the app region, CRC32 matches. Reject otherwise. (Never use a .syx for
      another model: Mini Mk3 = 0x0D, Pro Mk3 = 0x0E.)

Gate 1 (hardware, no flashing yet):
- [ ] Connect the X, run `amidi -l` and note it appears as "Launchpad X".
- [ ] Practice the bootloader entry (unplug, hold Capture MIDI, plug in) and
      confirm the "Launchpad MIDI Bootloader" port appears, with the device
      on firmware that is currently working. Unplug to leave (do not flash).
- [ ] Practice re-flashing the STOCK file (LPX-351.syx) through the bootloader.
      Confirm the device boots. This proves the recovery path end to end.
      Do this BEFORE flashing any custom image.

Gate 2 (first custom flash):
- [ ] Flash an unmodified `cargo lpx` CoreFW image first (known upstream,
      same code path as everyone else), not our Seq95 build.
- [ ] Confirm it boots; enter bootloader again; re-flash stock. Confirm recovery.

Gate 3 (our build):
- [ ] Only now flash the Seq95 build. After each flash, verify boot, then verify
      bootloader entry still works (power-cycle with Capture MIDI held).
- [ ] Seq95 v1 keeps projects in RAM only. No external-flash writes until the
      persistence layer is separately reviewed (a bad SPI flash write can
      corrupt the settings area, but cannot touch the MCU bootloader).

STOP conditions: any unexpected mismatch in the gates; any need to change
memory.x; the device not showing the bootloader port at Gate 1. Then ask.

## 5. Recovery procedures

A. App misbehaves but the device runs: hold Capture MIDI, replug, flash stock
   LPX-351.syx with `core/tools/flash.py` or fw.mat1jaczyyy.com (Chrome/WebMIDI),
   or use Novation Components.
B. App does not boot (dark pads): same, bootloader entry is a hardware key
   check. If the bootloader port does not appear, try another USB cable/port
   and a different host before concluding anything. Do not keep reflashing blindly.
C. Bootloader port never appears: that is the unrecoverable case (needs SWD
   programmer on the STM32 pads). This is why Gates 1 and 2 exist.

## 6. Build status (updated)

- Tooling: `arm-none-eabi-objcopy` installed (binutils-arm-none-eabi). Use
  `PATH=$HOME/.cargo/bin:$PATH` (system cargo 1.85 is too old).
- Port branch: `src/mod`, branch `lpx-seq95`, on top of drusanow commit 109adc4.
  Changes so far: Seq95 compiled in for `launchpad-x` (Cargo.toml feature,
  cfg gates in app/mod.rs and app/host.rs), a storage stub that refuses all
  saves/loads (no external-flash access on the X), and `opt-level = "z"` for
  the seq95 crate only.
- Result: the linker enforces the 80 KB region (the first attempt overflowed by
  7.7 KB and failed to link, which is the safe outcome). After the size
  optimization: image ends at 0x0801EAB0, about 5.4 KB below 0x08020000.
  `cargo lpx` produces build/lpx/fw.bin (76,464 bytes) and fw.syx.
- Control remap DONE (seq95/src/xmap.rs, unit-tested on the host, 28 tests pass).
  Deviation from the proposal: Shift is Capture MIDI (98), not Session, because
  the host already consumes Session (95) for the 500 ms Setup hold. Capture MIDI
  is only a bootloader key while powering up; at run time it is a normal button.
  Final map: Up/Down = prev/next track (Shift: transpose +-12), Left/Right = page
  (Shift: tempo), Note = Play, Custom (97) = enter/leave Seq95, Capture = Shift.
  Right column = lanes; with Shift: Record, Clear, Duplicate, Fixed length,
  Mute, Solo, Projects. Clear and Mute are holds (step, page or voice); a tap
  without touching the grid applies to the whole current track. Solo is immediate.
- LED writes are confined to X indexes (< 100) by construction and by test.
- Gate 0 verifier: tools/verify_lpx.py (ELF segments inside the app region,
  margin >= 2 KB, vectors, syx product id 0x0C, syx decodes back to fw.bin).
  Current build: PASS, image ends 0x0801F178, margin 3720 bytes. The stock
  LPX-351 files also PASS, which validates the verifier itself.
- Pro Mk3 build still compiles. Storage on the X refuses all saves (RAM-only).
- NOT yet done: any hardware gate (section 4). Not flashed. Behavior on real
  pads is untested (no way to test on host beyond unit tests).

## 7. Port plan

1. DONE: take the `seq95` crate unchanged; build for lpx; RAM-only storage.
2. DONE: control remap. Seq95 needs about 21 control buttons plus 8 lane buttons;
   the X has 16 side buttons. Proposal: right column = 8 lanes (as now); top row
   = Up/Down/Left/Right, Session = Shift, Note = Play, Custom = enter/leave
   Seq95, Capture MIDI left untouched; Record/Clear/Duplicate/Quantise/
   Fixed length/Mute/Solo/Projects as Shift + right-column buttons.
3. DONE: LED/index translation layer (110-LED Pro space to the X's 100 LEDs),
   with bounds checks that fail loudly (no silent clamping).
4. DONE: Gate 0 verifier. NEXT: hardware gates in section 4.
5. LATER: persistence in a reserved block of the 1 MB flash below the settings.

## 8. Extra offline evidence (added before first hardware use)

Facts gathered without the device:
- Novation's Launchpad X Programmer's Reference confirms the bootloader is a
  separate interactive mode: entered with Capture MIDI held while plugging in;
  it has its own pad surface (USB device ID, bootloader/application version in
  the lower-left green pads, one green pad at the bottom right that starts the
  application) and answers a Device Inquiry with its own identity
  (F0 7E 00 06 02 00 20 29 13 11 ... F7, versus 13 01 for the application).
  So the entry is a property of the bootloader, not of the application image.
- Our .syx has exactly the same message structure as the stock LPX-351.syx and
  as the upstream CoreFW lpx .syx: 1 init (0x71, family 02, product 0C), 1 header
  (0x7C), N write blocks (0x72), 1 finish (0x73). Only the block count differs.
- Our image ends at 0x0801F178. The STM32F401 sectors there are: 3 =
  0x0800C000-0x0800FFFF (16 KB), 4 = 0x08010000-0x0801FFFF (64 KB), 5 starts at
  0x08020000. Stock (68 KB) already needs sectors 3 and 4; ours uses the same
  two sectors and stays below sector 5. That is also why CoreFW's 80 KB limit is
  exactly 0x14000: it is sectors 3 + 4. So no erase outside the stock set is needed.
- The boot_version block at 0x0800C1E8 in our image is byte-identical to the
  one in upstream CoreFW lpx (only the version digits differ from stock).
- No public report of a Launchpad X brick by custom firmware was found.

Still NOT provable offline (needs the device):
- That the bootloader still starts when the application is broken.
- How the bootloader reacts to an update larger than any stock image we have
  (ours 78,200 bytes; stock 351 is 68,282; the oldest LPX 236/238 were 75.8 KB;
  upstream CoreFW 57,848). It stays inside the same two sectors, which is the
  best offline argument, not a proof.
- Whether an SWD recovery route exists (option bytes, test pads). Unknown.

New non-destructive Gate 1a (before Gate 1, flashes nothing):
- Enter the bootloader (unplug, hold Capture MIDI, plug in). Read the version
  pads; send a MIDI Device Inquiry and confirm the 13 11 identity. Press the
  green start pad and confirm the stock application starts. Unplug.
  This proves entry and the start-application path with zero writes.
- Optional: send an incremental check - flash CoreFW first (Gate 2) before ours,
  so the biggest unknown (a 78 KB image) is the last step, not the first.

Cheap insurance to prepare in advance (not required, but it is the only fix for
the one unrecoverable case): an ST-Link or other SWD programmer, and photos or
a teardown of the board to find the SWD pads. Do not open the device or
change option bytes unless the bootloader is already dead.

## 9. Findings from the attached device (read-only, 2026-10-29)

Only reads were done: lsusb and one universal MIDI Device Inquiry to each port.
Nothing was written, no bootloader entry, no flashing.
- USB 1235:0103 "Launchpad X", bcdDevice 2.00, serial <serial omitted>, ports
  "LPX DAW" and "LPX MIDI".
- Device Inquiry reply (both ports): F0 7E 00 06 02 00 20 29 03 01 00 00
  00 04 02 02 F7. The application version bytes are 0,4,2,2 = build 422.
- IMPORTANT: the installed application is build 422, NEWER than the stock
  LPX-351 we hold. Flashing LPX-351 back would be a downgrade, not a restore.
  The correct recovery is Novation Components (current release). We have no
  copy of build 422 and cannot read it back from the device. Do not treat
  LPX-351.syx as "the" rollback; it is a last-resort fallback only.
- The family bytes are 03 01, while the Programmer's Reference prints 13 01
  (application) and 13 11 (bootloader). The manual's identity values are
  therefore not reliable here; at Gate 1a expect something like 03 11 for the
  bootloader and only compare against what we actually see.
- There is no known way to read the application or bootloader flash back over
  USB, so a firmware backup is not possible. Only device-specific data is
  recoverable by the user (Custom Mode layouts via Components, if any).

Before any flashing: download the current firmware with Novation Components
(web app) and keep its .syx if it offers one, and export Custom Modes.

## 10. Gate 1a result: bootloader entry confirmed (2026-10-29)

Read-only again: lsusb, amidi -l, one Device Inquiry. Nothing written.
- Holding Capture MIDI while plugging in works: the USB identity changes to
  1235:1103 "Launchpad X Bootloader" (serial unchanged, <serial omitted>),
  with a single MIDI port "LPX BL M".
- Device Inquiry reply from the bootloader: F0 7E 00 06 02 00 20 29 03 11 00 00
  00 02 03 08 F7. Family bytes 03 11 (as predicted: the app answers 03 01),
  bootloader version bytes 0,2,3,8 = bootloader 238.
- The bootloader enumerates on its own USB descriptor, so entry is not
  something the application provides. Still unproven: that it starts when the
  application is broken (that needs a bad or partial image, which we will NOT
  create deliberately).

Start-application path confirmed: after pressing the green start pad the device
re-enumerated as 1235:0103 "Launchpad X" and answered the Device Inquiry with
03 01 ... 00 04 02 02 (application 422), unchanged. Gate 1a: PASS.
Next: Components check (obtain the current official firmware), then Gate 1 / 2.

Components (user report): the unit shows firmware 2.0.1 installed and "up to date".
This is the same release as the application build 422 read from the Device
Inquiry (inferred from the two matching, not stated by Components). LPX-351 is
older than it, so flashing it is a downgrade until Components updates the unit.

## 11. First flash attempt with the web tool: red F (user report, 2026-10-29)

- fw.mat1jaczyyy.com (custom SysEx path, stock LPX-351.syx) ended every time in
  a red F on the Launchpad (user report). The unit stayed in the bootloader
  (1235:1103 enumerated). Novation Components, on another USB port, then
  reinstalled Novation firmware successfully and the unit is back in normal
  mode (1235:0103). Installed version not re-read yet (MIDI ports were busy).
- Evidence this gives: (a) a failed transfer did NOT brick the unit, the
  bootloader survived and a normal Components update recovered it; (b) the
  bootloader itself seems to judge the update and signal failure with a red F.
- The web tool sends the whole file with a 1 ms timer per message and waits
  for no acknowledgement (classes/Launchpad.ts flashFirmware); CoreFW flash.py
  is equally open-loop. Neither can tell us why the bootloader said F.
- Hypotheses for the F, not yet separated:
  1. Transfer problem (pacing/dropped messages on this host or browser).
  2. The bootloader refuses an image OLDER than the installed one (downgrade
     protection): LPX-351 < installed 2.0.1 (build 422). If so, our CoreFW
     image, which claims version 351 in its syx header, would be refused too.
  3. A problem with the file/tool. LPX-351.syx decodes and checks out
     offline, so this is the least likely.
- Rule from this point: no further flash attempt until the cause is known.

### 11a. Cause narrowed (2026-10-29)
- User: the red F is on the Launchpad pads and appeared at the END of the
  progress bar, i.e. after the whole file was sent. So the bootloader accepted
  the start of the update and judged the finished image bad.
- Upstream CoreFW issue #9 (open): "Web flasher fails on Linux", Launchpad X,
  blinking red F, works on Windows. Maintainer: the SysEx update seems to be
  sent in the wrong order on Linux; workaround: offline tools/flash.py.
- Conclusion: most likely a transfer problem (ordering/loss in the Linux
  browser MIDI path), not a downgrade refusal and not a bad file. Not proven.
  The bootloader verifies the complete image before accepting it, which is
  the behavior we want: a bad transfer produces F instead of a broken app.
- Next safe step: resend LPX-351.syx with a sender that cannot reorder (ALSA
  rawmidi via amidi, a byte stream), reading the file sha256 first. A failure
  is expected to be harmless again (F, bootloader stays).

Check after the failed web flash and the Components reinstall (read-only, Brave
closed): the application answers 03 01 ... 00 04 02 02 on both ports, i.e.
build 422 (2.0.1) is installed again. The unit is fully restored.

## 12. Gate 1b PASS: stock LPX-351 flashed with amidi (2026-10-29)

- Bootloader confirmed first (1235:1103, identity 03 11, bootloader 0238).
- sha256 of LPX-351.syx verified, then sent with
  `amidi -p <bootloader port> -i 10 -s LPX-351.syx` (28 s, exit 0, 2136 messages).
- Result: the unit restarted by itself as 1235:0103; Device Inquiry now answers
  application version bytes 0,3,5,1 = build 351 (was 0,4,2,2 = build 422).
  The user confirmed the update succeeded (no red F).
- Conclusions: (1) the earlier red F was the Linux browser transfer path, not
  the file; (2) the bootloader ACCEPTS a downgrade (422 -> 351), so there is
  no downgrade protection that could reject an image claiming version 351;
  (3) amidi with a 10 ms interval is a working, ordered sender on this host.
- The unit now runs build 351, not 2.0.1. Restore with Novation Components.

## 13. Gate 2: upstream CoreFW flashed (2026-10-29)

- Built from the cloned upstream source (commit 3f5e353, unmodified), Gate 0
  verifier PASS, sha256 0ac88499...d01988 verified, device in the bootloader
  (identity 03 11, bootloader 0238).
- Sent with `amidi -p hw:1,0,0 -i 10 -s core-launchpad-x.syx`, exit 0.
- The unit re-enumerated by itself as 1235:0103, MIDI ports renamed "LPX (DAW)"
  and "LPX (MIDI)" (CoreFW's own descriptors), Device Inquiry answers
  application version bytes 0,9,9,9 (CoreFW's version marker).
- Still to do for the gate: user confirms the pads/boot animation, then the
  bootloader is entered again (Capture MIDI at plug-in) and a stock restore is done.

Gate 2 re-entry check: with upstream CoreFW installed, Capture MIDI at plug-in
still gives 1235:1103 "Launchpad X Bootloader", identity 03 11, bootloader 0238
(same as before). Entry works with custom firmware installed. Read-only; no flash.

## 14. Gate 2 complete: restored to stock 351 from CoreFW (2026-10-29)

- From the bootloader (CoreFW installed before), sha256 of LPX-351.syx verified,
  `amidi -p hw:1,0,0 -i 10 -s LPX-351.syx`, exit 0.
- The unit re-enumerated as 1235:0103 with the Novation port names "LPX DAW" /
  "LPX MIDI" and answers application version 0,3,5,1 (build 351).
- Whole round trip proven on this unit: 422 (2.0.1) -> 351 -> upstream CoreFW
  999 -> bootloader re-entry -> 351. Remaining: Components update 351 -> 2.0.1
  (not required before Gate 3, but it is the official restore; do it at the end).
- Gate 3 (our Seq95 build, 78,200 bytes) is NOT yet done.

## 15. Gate 3: Seq95 build flashed (2026-10-29)

- Commit 704f5fa, rebuilt with `cargo lpx`, Gate 0 verifier PASS (ends 0x0801F178,
  margin 3720), sha256 c6def0c9...fd26 verified, device in the bootloader
  (identity 03 11, 0238).
- `amidi -p hw:1,0,0 -i 10 -s fw.syx`, exit 0; the unit re-enumerated as 1235:0103,
  ports "LPX (DAW)" / "LPX (MIDI)", Device Inquiry 03 01 ... 00 09 09 09 (CoreFW).
- Pending: user checks boot and the sequencer on the pads; bootloader re-entry
  check with this build; Components restore to 2.0.1 at the end.

## 16. Is flash above the image free? (offline analysis of stock LPX-351, 2026-10-29)

Method: disassembled LPX-351.bin (arm-none-eabi-objdump, thumb) and searched for
peripheral base addresses and for pointers into 0x08020000-0x0807FFFF.
- The only users of the internal FLASH controller (0x40023C00) are two clock/
  startup routines that set FLASH_ACR (2 wait states, prefetch, caches). There is
  no unlock (KEYR), erase or program sequence: the stock application never
  writes internal flash.
- No literal pointers into sectors 5-7 (0x08020000-0x0807FFFF). The few matches
  are Thumb-2 instruction halfwords, not addresses.
- The application uses SPI2 and SPI3 (SPI2 = the external flash, as in CoreFW).
  Settings the application changes (brightness) are therefore stored outside the
  internal flash, or handed to the bootloader some other way.
- Conclusion (inference, not proof): the application does not use sectors 5-7.
  Whether the BOOTLOADER keeps data there (calibration, serial, its own
  settings) cannot be seen from the application and is unknown.
- Whether the bootloader accepts an image larger than sector 3+4 (80 KB) is
  unknown and can only be tested on the device. Probe design if ever needed:
  stock 351 padded with 0xFF past 0x08020000; worst case F (harmless, seen
  before); if accepted it erases sector 5 (risk: the unknown bootloader data).
  Not needed for the current Seq95 build (3.7 KB margin); do not run it unless a
  feature requires more than 80 KB.

## 17. Stock LPX-351 structure (offline, Thumb disassembly, 2026-10-29)

Method: arm-none-eabi-objdump -D thumb on LPX-351.bin at 0x0800C000 (the Ghidra
session loaded it as ARM, not Thumb, so it was dropped; the disassembly is the
source here). Compiler looks like Keil/armcc (scatter-load runtime, semihosting bkpt).
- Vector table: SP 0x2000FFF8, Reset 0x0801C86D. Reset -> SystemInit (0x0801C49C:
  FPU/clock regs) -> __main (0x0801C888 enables the FPU, 0x0801C804 init) -> main
  0x0800C378.
- Real interrupt handlers (STM32F401 numbering): SysTick -> tick counter at
  0x0801C718 (adds a byte to a RAM counter); TIM1_UP_TIM10 -> 0x0800C32E; TIM2 ->
  0x0800C33C; OTG_FS (IRQ 67) -> USB stack entry 0x08016D78. About 50 other
  vectors are `b .` loop stubs (unused). HardFault etc. are `b .` (hang), NMI/SVC/
  PendSV return.
- main 0x0800C378: peripheral init, a large state struct at RAM 0x200009EC (fields
  up to +0x11D0), USB/MIDI engine init, then a super-loop at 0x0800C522 calling
  ~15 poll functions (0x0800F60E, F6C6, D952, E3A2, E87C, F0AC, EDFA, F734, D898,
  DD66, D7E6, DAAC) and then draining a message queue (0x0800CF6C, CFC0, D020, E7D4).
  Button/mode dispatch and LED writes must sit in or under those functions; not yet traced.
- Hook candidates for an injected sequencer: (1) the SysTick handler for the clock,
  (2) the loop head 0x0800C522 for UI/LED work, (3) the event dispatch (to find),
  (4) LED buffer ownership (to find).
- IMPORTANT for safety: the 24 bytes at 0x0800C1E8 (E7 D1 30 BC 70 47 01 20 70 47
  FF FF FF FF FF FF "00351") are reproduced by CoreFW's version.x, and the ASCII
  "00351" there is evidently what the bootloader shows as the application version.
  The bootloader probably also validates something around that block. NEVER change
  this block in any image; our builds keep it byte-identical to upstream CoreFW.
- RAM: the stock code references addresses across the whole 64 KB (stack at the
  top); free RAM cannot be derived from literals, needs the scatter-load table.
- Version: the unit shipped with build 422 (2.0.1); only 351 is analyzed. Any
  injection would target 351, or needs the 422 image (not obtainable by reading).

## 18. Where the bootloader lives and what could overwrite it (2026-10-29)

- The Novation bootloader is SOFTWARE in the STM32F401's internal flash,
  sectors 0-2 (0x08000000-0x0800BFFF), not hardware/mask ROM. The chip also has
  ST's factory ROM bootloader (system memory 0x1FFF0000, USB DFU), which cannot be
  overwritten, but it only starts when the BOOT0 pin is held high at reset: needs
  opening the device and a pad/jumper (unknown on this board). If RDP is set,
  DFU still allows a mass erase + reprogram, but that wipes the Novation bootloader.
- Overwrite paths: (1) a .syx update: it carries no addresses (only size and CRC),
  the bootloader writes it upward from 0x0800C000, so it cannot reach sectors 0-2
  unless the bootloader itself is buggy; (2) code running in the application that
  erases/programs internal flash. Checked offline: stock 351, upstream CoreFW lpx
  and our Seq95 build contain NO FLASH_KEYR/OPTKEYR unlock constants and NO
  reference to the FLASH controller (0x40023C00) as literals or movw/movt pairs
  (579/677 constants decoded for CoreFW/ours; stock sets only ACR). They cannot
  unlock internal flash. (3) A physical SWD/DFU tool: not used.
- If the bootloader were destroyed: USB recovery would be gone. Only DFU/SWD on
  the board could help, and we hold NO copy of the Novation bootloader to restore.
  A donor unit's dump (SWD) or Novation service would be needed.

## 19. Build with Note Mode (2026-10-30, NOT flashed yet)

- New crate notemode/ (11 host tests pass), app glue src/app/note/mod.rs, Note
  button hook in the host (Performance mode only), AppId::Note. User guide and
  rationale: docs/note-mode-launchpad-x.md.
- Size: the first link failed by 96 bytes (the linker refuses an image that
  does not fit the 80 KB region: safe). To make room the seq95 crate got a
  "persistence" feature (project encode/decode, CRC32, Projects overlay), off for
  the X (the X refused every save anyway), on for the Pro Mk3. This also frees
  about 7 KB of RAM (the project scratch buffer). notemode is built with
  opt-level "z".
- Result: image ends at 0x0801F9A0, margin 1632 bytes (before: 3720). The
  verifier margin policy was lowered from 2048 to 1024 bytes: the hard limit is
  enforced by the linker; the margin only guards against drift. Gate 0: PASS.
  sha256 of build/lpx/fw.syx: aed00cb475eeeea4e56cfdaa29a5534f295c07232273384735d3bdb33043339a
  (2511 messages, product 0x0C).
- Tests: seq95 28 (persistence on) and builds clean with persistence off;
  notemode 11; Pro Mk3 firmware still builds.
- Behavior change on the X sequencer: Shift + Probability (Projects overlay) now
  does nothing.
- This image has NOT been run on a device. Before flashing: bootloader entry
  check (read-only), then flash with amidi as before. Recovery unchanged: flash
  LPX-351.syx from the bootloader, then Components.

## 20. Note Mode build flashed (2026-10-30)

- Commit 84f9879 rebuilt, Gate 0 PASS (ends 0x0801F9A0), sha256 aed00cb4...3339a
  verified, device in the bootloader (03 11, 0238), `amidi -i 10 -s fw.syx`, exit 0.
- The unit re-enumerated as 1235:0103, ports "LPX (DAW)" / "LPX (MIDI)", Device
  Inquiry 03 01 ... 00 09 09 09. Pending: user checks Note Mode on the pads.

## 21. Size optimization for the X build and free storage (2026-10-30)

- `cargo lpx` (xtask) now builds the X with CARGO_PROFILE_RELEASE_OPT_LEVEL=s
  (the seq95 and notemode crates stay at "z"). Other devices keep opt-level 3.
  A plain `cargo build --features launchpad-x` still uses opt-level 3 and fails to
  link (image too large): build the X only with `cargo lpx`.
- Measured (scratch copy, bigger region): level 3 = 80.2 KB, s = 58.6 KB, z = 53.4 KB.
- Result: fw.bin 58,368 bytes, image ends 0x0801A400, margin 23,552 bytes.
  Gate 0 PASS, 1826 syx messages, sha256 b1c32402cf88b3f4968af523b63afea4ad31631e3590822a31754d790ddf9d97.
  Risk is behavior, not bricking: pad scanning and LED refresh timing was tuned
  for speed upstream and is untested at -Os. NOT flashed yet.
- Free storage search (FF or 00 runs of at least 32 bytes inside the images):
  stock 351 has 3 (36, 36, 102 bytes of 00), upstream CoreFW and ours one
  (84 bytes of 00 at 0x0800C194, padding before the boot_version block).
  No usable padding. The only real free internal flash is the tail of sector 4,
  0x0801A400-0x0801FFFF (23,552 bytes), erased by the bootloader before an update
  (inferred, never read back). Sectors 5-7 (0x08020000-0x0807FFFF) are unknown
  (see section 16). External SPI flash (1 MB): content unknown; the host cannot
  read the Launchpad's flash, only firmware running on it could.

## 22. Read-only flash diagnostic (2026-10-30, built, NOT flashed yet)

- Purpose: map erased/unused areas of the internal flash (including a backup of
  the bootloader) and the external SPI flash before anything is ever written.
  Docs: docs/flash-diagnostic.md. Firmware: src/sys/diag.rs, feature `diag`,
  built only by `cargo lpxdiag`; host: tools/lpx_dump.py; test: tools/diag_host_test.
- Gate 0: image ends 0x0801A8C8, margin 22,328 bytes, PASS,
  sha256 818aa2741fed0df76a17c8daf09c1dfe3af00a1cb86b6226da370c655e8a38fb (1865 messages).
  The normal lpx image is unchanged (sha256 b1c32402...9d97).
- Offline check: neither the diag nor the normal -Os image contains flash unlock
  keys or a reference to the FLASH controller (1731 and 1699 movw/movt constants decoded).
- Host test PASS. It caught a wrong request-length check (+9 instead of +8) that
  would have answered every read with an error.
- Risks: it adds SysEx handling on the MIDI port for the diag build only; reads
  of the external SPI flash take the same SPI bus the settings use (read-only).

## 23. Results of the internal flash dump (2026-10-30, diag build, read-only)

Dumped over MIDI at about 53 KB/s; the application region of the dump is byte-
identical to the flashed firmware file (this proves the dump path end to end).
Local copies (NOT committed: Novation's code) in ../dumps/: 0-internal.bin
(sha256 starts 1a3b2606), 2-otp.bin, 3-option-bytes.bin, 4-system-rom.bin.
**Keep a second copy of 0-internal.bin: its first 48 KB are the only backup of the
Novation bootloader that exists.**

- **The chip has 128 KB of flash, not 512 KB.** The size register at 0x1FFF7A22
  reads 128 KB, so the flash ends at 0x08020000 and sectors 5-7 do not exist.
  This corrects the assumption in sections 7 and 16 ("free flash up to
  0x08080000"): there is no flash above the application area. The application area
  is sectors 3 (16 KB) and 4 (64 KB), 0x0800C000-0x0801FFFF. (CoreFW builds with
  the stm32f401re feature; the chip itself behaves like a 128 KB part.)
- **Option bytes** (16): ef aa 10 55 ef aa 10 55 ff 7f 00 80 ff 7f 00 80. Byte 1 =
  0xAA: read-out protection level 0 (none). This is why the CPU and a debug probe
  can read everything. Write protection words ff 7f (complement 00 80) are not
  fully interpreted; no sector write-protection was assumed or relied on.
- **Bootloader** 0x08000000-0x0800A0BF (40,128 bytes used; vector table SP
  0x2000FFF8, reset 0x08009D1D; version text "000238" at 0x080001F8; strings
  "Launchpad X Bootloader", "LPX Bootloader", "Onboarding", a "Launchpad Mini MK3"
  string: one code base for both devices). 0x0800A0C0-0x0800BFFF (8,000 bytes) are
  FF but belong to the bootloader's sector 2: they cannot be erased without
  destroying the bootloader, so they are not usable.
- **Validity record written by the bootloader.** The last 16 bytes of flash,
  0x0801FFF0-0x0801FFFF, hold four words: 0, version (0x351 = the number given to
  syxtool), image length (0xE8C8 = 59,592 = our fw.bin) and a CRC32 of the image
  (polynomial 0x04C11DB7, MSB first, init 0xFFFFFFFF, no final xor; reproduced
  exactly). Interpretation (strong, not proven): the bootloader checks the whole
  received image against the CRC from the update header and writes this record
  only on success. That explains the red F after a bad transfer and why an
  incomplete update cannot produce a valid-looking application. The location is
  the end of flash, so NEVER write to 0x0801FFF0-0x0801FFFF.
- **Free internal flash:** after the image, 0x0801A8A0-0x0801FFEF is FF (22,352
  bytes), but it lies in the sectors that every firmware update erases, so
  anything stored there is lost at the next update. Not suitable for projects.
- OTP is erased (all FF, 560 bytes read); ST's system ROM bootloader (30 KB) was
  read, not studied.
- **External SPI flash:** JEDEC C2 20 14 (Macronix, 8 Mbit = 1 MB). The firmware's
  driver exposes only an 8 KB window (the last 8 KB) so the first diag build could
  not read the whole chip; a physical read path was added (read-only, same
  protocol). Pending: reflash the corrected diag build and dump the chip.

## 24. Results of the external SPI flash dump (2026-10-30, read-only)

Chip: Macronix, JEDEC C2 20 14, 1 MB. Whole chip read in 4 KB sectors; sha256 of the
dump starts c7534c2e. The internal dump of the same run again matches the flashed
firmware byte for byte. Local copies in ../dumps/run2/ and ~/lpx-backup/run2/ (not
committed).

Map (offsets in the chip):

| Range | Size | Content | Verdict |
| --- | --- | --- | --- |
| 0x000000-0x001FFF | 8 KB | Identity block: "Focusrite Novation", "Launchpad Mini MK3" (shared factory data), the serial number <serial omitted> at 0xC0 | Never touch |
| 0x002000-0x002FFF | 4 KB | FF | Unknown, leave alone |
| 0x003000-0x011FFF | 60 KB | **The firmware image last received, byte for byte** (the diag image, 59,552 bytes, starts at 0x3000) | Bootloader staging area, rewritten at every update |
| 0x012000-0x01FFFF, 0x020000-0x038FFF | | Remnants of earlier, larger images and tables | Part of the staging area |
| 0x039000-0x03FFFF | 28 KB | zeros | Staging area |
| 0x040000-0x0FDFFF | 760 KB | **all FF (erased)** | Candidate for saved data, see below |
| 0x0FE000-0x0FEFFF | 4 KB | CoreFW settings ("COREFW" header) | CoreFW's, keep |
| 0x0FF000-0x0FFFFF | 4 KB | FF | CoreFW's settings window (8 KB: 0xFE000-0xFFFFF) |

How an update works (derived from the dumps): the bootloader writes the received
SysEx blocks into the external flash at 0x3000, checks them against the CRC of the
update header, and only then copies the image into the internal flash and writes
the 16-byte validity record at the end of the internal flash. A failed transfer
therefore stops in the staging area: the internal application is not touched, which
matches the red F seen earlier and is the best explanation for why a bad flash has
not bricked the unit so far.

Where could data be written SAFELY (for the Seq95 projects)?
- **Best candidate: a block at the high end of the erased area, for example
  0x0F0000-0x0FDFFF (56 KB)**, directly below CoreFW's own settings, 700 KB away
  from the staging area, and erased. Writing there cannot touch the bootloader's
  identity block, the staging area, the internal flash or CoreFW's settings.
- Not established: whether Novation's own firmware stores anything in the 760 KB
  area (for example Custom Modes uploaded with Components; this unit has none,
  which is why it reads FF). A literal search in the stock image gave no usable
  evidence (the matches were Thumb-2 instruction encodings, not addresses).
  Consequence if it does: a data collision between the two firmwares, not a brick.
  Mitigation: use a signature header and a CRC for every record, never erase
  outside our own sectors, and keep the block small.
- Writes must stay aligned to the chip's 4 KB sector erase unit and inside the
  chosen block; the driver's existing write path (read-modify-write with a sector
  buffer) already works that way for CoreFW's settings.
- Do not write to 0x000000-0x03FFFF (identity and staging) or to the last 16
  bytes of the internal flash.

## 25. Chord, Custom Modes and Session tap (2026-10-30, built, NOT flashed)

- New: Chord layout in notemode (19 tests), crate custommode (12 tests, eight
  built-in Custom Modes), Custom app and routing in the host: Custom opens the
  Custom Modes, Capture MIDI + Custom the sequencer, a tap on Session opens the
  DAW (Session) mode while a hold still opens Setup. Docs: launchpad-x-modes.md,
  custom-modes-launchpad-x.md, custom-mode-format-notes.md.
- Components Custom Modes are NOT implemented: the format is only known from a
  third-party tool's source and the exchange with Components is unknown.
- Size: image ends 0x0801C1F8, margin 15,880 bytes (before: 21,232). Other
  devices still build.
- Behavior changes vs the build on the device: Custom no longer opens the
  sequencer (use Capture MIDI + Custom); the layout pads on the Note settings
  page moved to the bottom row.

## 26. Chord / Custom Modes / Session tap build flashed (2026-10-30)

- Commit f88f711 rebuilt with `cargo lpx`, Gate 0 PASS (ends 0x0801C1F8, margin
  15,880), sha256 9c3505ea...474d verified, device in the bootloader (03 11, 0238),
  `amidi -i 10 -s fw.syx`, exit 0. The unit re-enumerated as 1235:0103, ports
  "LPX (DAW)" / "LPX (MIDI)", Device Inquiry 03 01 ... 00 09 09 09.
- This is the first run of the size-optimized firmware (opt-level s) on real pads
  together with Note Mode, Chord, Custom Modes and the Session tap. Pending: user
  checks pad response, LEDs and the new modes.

## 27. Note Mode as startup mode (2026-10-30, built, NOT flashed)

- User report on the previous build: no way back to Note Mode, and the unit did not
  start in it (after boot it went to the plain Performance mode with dark pads).
- Change: the boot animation now ends in Note Mode on the X (boot/mod.rs,
  FIRST_APP); the Note button also works from Custom and Session (DAW) mode, not
  only from Performance. Not from Programmer mode (host software) or the sequencer
  (Play).
- Gate 0 PASS, image ends 0x0801C200 (margin 15,872), sha256
  24024c7df68773f1d4cb9a6702f86f609c9d3636651d832c64e548e4ac910c63. Other devices
  still build.

## 28. Note-Mode-at-startup build flashed (2026-10-30)

- Commit 362458a. Gate 0 PASS (ends 0x0801C200, margin 15,872), sha256
  24024c7d...0c63 verified, bootloader 03 11 / 0238, `amidi -i 10`, exit 0; the unit
  re-enumerated as 1235:0103 and answers 03 01 ... 00 09 09 09.
- Note for the procedure: a first Gate 0 run failed ("segment outside app region")
  only because the ELF in target/ had been overwritten by building other devices
  (mini, pro-mk3, mk2) after `cargo lpx`. Always run `cargo lpx` right before
  verify_lpx.py; after rebuilding, the same hash passed. The check did its job: it
  refuses an ELF that does not belong to the .syx.

## 29. Documentation overhaul (2026-10-30)

- New user manual docs/user-manual.md (mere-mortal level, all modes, Setup, MIDI,
  installing, going back, troubleshooting, what has been tested) with 14 pictures
  in docs/img/, drawn from the real code by tools/framedump (Rust) and
  tools/make_pictures.py. Index in docs/README.md; status lines of the per-mode
  documents updated to the real state (flashed, not yet confirmed mode by mode).
- Facts about Setup, Performance and Programmer mode were read from the source.
  Not tried by us: Ableton Live Session mode, the palette editor, Windows/macOS
  installation.

## 30. Saving data (2026-10-30, built, NOT flashed, NOT yet run on a device)

- New: crate safestore (12 host tests incl. power cut at every step), bounds-checked
  window in the chip driver (window 0x0DA000-0x0FDFFF, 36 sectors; compile-time
  asserts), src/sys/store.rs, preferences in Note and Custom Modes (22 and 14
  tests), the sequencer Projects overlay and storage on the X (8 slots). Design and
  safety argument: docs/storage-design.md. Verifier: tools/verify_store.py.
- Baseline: in the dump taken on 2026-10-30 the whole window (0x0DA000-0x0FDFFF) is
  erased (all FF), and all items read as empty.
- Image: normal build ends 0x0801CF30 (margin 12,496 bytes; saving adds about 3.4 KB);
  diag build ends 0x0801D420 (margin 11,232). Gate 0 PASS for both. normal sha256
  43a590742d75d071f304c6163c1f1f0e436640dcf22a494b9582422287b1f424; diag sha256
  08fea7c0037cfc30d900a5c7cc52a580aa96195c3f43f64507e6416505829136.
- Checked offline: no flash unlock keys and no FLASH controller references in the new
  image (the internal flash, where the bootloader lives, cannot be written); only
  the sector erase, page program and read functions of the chip driver are linked
  (no chip erase, no block erase).
- Plan for the first hardware test (the Launchpad was not attached to this machine
  when the work was finished):
  1. flash the diag build, dump the whole external flash (baseline);
  2. flash the normal build, save a project, change a Note Mode setting and a Custom
     channel, unplug and check they come back;
  3. flash the diag build again, dump, `verify_store.py` (items valid) and
     `verify_store.py --diff baseline after` (only the store window, CoreFW settings
     and the staging area may differ; identity block unchanged);
  4. record the result here.
- Residual risks: unbounded busy-wait in the chip driver (a hang, recoverable through
  the bootloader, not a brick); Novation's own firmware might use the window (data
  loss only); flash wear.

## 31. Incident: the saving build did not start (2026-10-30)

- What happened: the diagnostic build of the saving commit (9a61c05) was sent through
  the bootloader (transfer exit 0), but afterwards the Launchpad did not enumerate on USB
  (`device descriptor read/64, error -110`). It was recovered through the bootloader
  (Capture MIDI at plug-in) by flashing the earlier build 362458a, which started
  normally. No data was lost; the store window had not been written.
- Cause (found by reading the disassembly): a stack overflow at startup. The main task
  is polled by the executor and had a stack frame of 39,680 bytes (the temporaries of
  building the big statics, mainly the app host), but only 26,172 bytes of RAM were
  left above the static data after the saving feature added a 7 KB project buffer to
  the sequencer. The previous build had 35.8 KB free for a 32.5 KB frame: it worked,
  with a margin of only 3 KB. Nothing had checked this.
- Fix: (1) the big structures are built in functions of their own (`init_*` in the X
  driver), so the main task's frame is now 3,572 bytes; (2) the 7 KB project buffer is
  no longer inside the sequencer (and so no longer in the app host): it lives on the
  stack only while a project is saved or loaded; (3) projects are decoded in place
  without a 7 KB temporary. Result: static data ends at 0x20007DE4, 33,308 bytes free
  for the stack, startup needs 26,740 bytes (3,572 + 23,168), margin 6,568 bytes.
- New Gate 0 check in tools/verify_lpx.py: the free RAM must cover the startup frames
  with a margin of at least 4 KB, or the build is refused. It was verified in both
  directions: the fixed build passes (margin 6,568), the crashing build (9a61c05) is
  refused (margin -13,508).
- New builds (not flashed): normal sha256
  8a683c68cd869a6d84671bf1d77fbd939c531260532bc5cae29c1c9208cbeb5d (ends 0x0801CD38,
  margin 13,000); diag sha256
  3e68d65a5096427d066f0a1320cd918e1a40063804505ba171d6b949d9166088 (ends 0x0801D228).
- Lesson for the procedure: a successful transfer and a passing size check say nothing
  about whether the firmware runs. Because the bootloader is independent of the
  application, a build that does not start is recoverable, which is what the whole
  gate procedure relies on.

### 31a. The second failure, and why the first check was not enough (2026-10-30)

- The fixed build (3e68d65a, diag) failed the same way: after a clean transfer, dark
  pads and no USB (user report; `error -110`). A fault at startup stops the CPU in a
  `b .` loop, so neither the pads nor USB are serviced.
- My first check only counted the frames of the main task and of the `init_*`
  functions themselves, not what they call. The worst-case chain below
  `init_app_host` was 30,660 bytes (its 23 KB frame plus `Project::new`, 7.4 KB, which
  the sequencer's constructor runs on the stack): with the main task's frame that is
  34,252 bytes against 33,308 free, an overflow of 944 bytes.
- Fix: (1) the sequencer (the biggest app) is built in its own function and static
  before the app host, so their temporaries are never on the stack together; (2)
  `Project::new` builds the five tracks from one constant and patches them in place
  (7 KB of stack saved, with a test that it still equals the default tracks); (3)
  the Gate 0 check now builds the call graph from the disassembly and computes the
  worst-case depth below the main task (startup) and below every app handler (running,
  plus 2 KB for interrupts), requiring 4 KB of margin.
- Verified in both directions: the build flashed last (3e68d65a) is refused
  (startup needs 34,252 of 33,308: margin -944); the new builds pass (startup needs
  13,312, running needs 23,256 of 33,292 free: margins about 20 KB and 10 KB).
- New builds (not flashed yet): normal sha256
  2828fa7cb4ba2cf662b9e45b5ce651974877ffb0b8f5c91698736bded36d6f61 (ends 0x0801D3E8,
  margin 11,288); diag sha256
  6979b6093e9c848ee16636f3a6506e37e0b4ded3591d32f35ce59a49e3504167 (ends 0x0801D8D8).
- Limits of the check: indirect calls (trait objects) are not visible in a call graph,
  so the app handlers are used as extra roots; interrupt nesting is an allowance, not a
  measurement. A passing check lowers the risk, it does not prove that the firmware
  runs; only starting it does.

## 32. First real save verified on the device (2026-10-30)

- Sequence: baseline dump (diag build 6979b609, window completely erased, all items
  empty) -> flash of the normal saving build 2828fa7c (started normally) -> the user
  saved sequencer project 1 (the pad flashed white, i.e. the firmware's own read-back
  check passed) -> flash of the diag build again -> dump of the whole chip.
- Result of `verify_store.py` on the dump: project 1, bank A: valid, generation 1, 7147
  bytes, and the sequencer's own CRC inside the payload is correct. All other items
  empty.
- `verify_store.py --diff baseline after`: only two ranges changed, both in the store
  window: 0x0DA000-0x0DA01F (the 32-byte header) and 0x0DA100-0x0DBCEA (7,147 bytes of
  payload, exactly the size of a project), plus the update staging area (expected).
  Byte-identical before and after: the identity block (0x0000-0x2FFF), the unused area
  0x040000-0x0D9FFF, CoreFW's settings (0x0FE000-0x0FFFFF) and the bootloader area of
  the internal flash (0x08000000-0x0800BFFF).
- The first run of verify_store.py reported "header CRC wrong (torn write?)" for the
  valid record: the verifier read the header CRC from bytes 16-19 instead of 20-23
  (the firmware was right, the Python tool was wrong). Fixed, and it now also checks
  the sequencer's inner CRC.
- Not yet verified on the device: loading the project back after a power cycle, and
  the saving of the Note Mode and Custom Mode preferences (no preference item was
  written in this test).

## 33. Project load confirmed on the device (2026-10-30)

- The user power-cycled the Launchpad and loaded project 1 from the Projects overlay:
  the saved pattern came back. So saving and loading of a sequencer project work end to
  end on a real Launchpad X (save: white flash and a verified dump; load after a power
  cycle: confirmed by the user).
- Still open: the dump-level verification of the Note Mode and Custom Mode preference
  records (they were not part of the dump checks), and the other untried modes listed in
  the user manual (section 13).
- State at the end of the session: the Launchpad runs the normal saving build
  (sha256 2828fa7c...6d61, commit a3fce75 plus later documentation commits).

## 34. Setup preferences, Novation-style sequencer, Setup extras (2026-10-30)

- Store window grown from 36 to 38 sectors, downward: new item `setup preferences` at
  sectors 0-1; every older item kept its absolute position (host test
  `items_keep_their_place_in_the_chip_when_the_window_grew`, 14 safestore tests, power
  cut test for the new item). The window start moved from 0x0DA000 to 0x0D8000 and still
  ends at the CoreFW settings; still far above the staging area and the identity block.
- Build option `LPX_SEQUENCER=launchpad95|novation` (default launchpad95); exactly one
  sequencer is linked. The Novation-style sequencer (crate `novseq`, 58 host tests) keeps
  its project in zero-initialized RAM, uses the same safestore slots with its own
  magic ("NSQ" + version); a slot written by the other sequencer is refused, a damaged
  slot is refused before anything is changed.
- Setup extras: pad trigger and aftertouch thresholds, MIDI clock in/out gates, Vegas mode
  and LED sleep (LEDs scanned out dark, colors kept), saved with the setup preferences.
- Flashed and confirmed by the user on the device, in this order: novation S1, S2, full
  sequencer (S1-S5), Setup extras (sha256 26c660f9...). All passed `verify_lpx.py` (image
  margin 13 KB, stack margin above 4 KB) and the bootloader identity check (03 11, 0238)
  before each flash. No boot problems.
- Not yet dump-verified: the setup preference record and the novation project record
  (use the diag build and tools/verify_store.py).
