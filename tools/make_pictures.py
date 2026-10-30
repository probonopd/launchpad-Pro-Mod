#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Simon Peter
"""Draws the pictures of the manual (docs/img/*.svg).

The pad colors come from the real mode code: run the frame dump and pipe it in.
  cd tools/framedump && cargo run --quiet | python3 -I ../make_pictures.py ../../docs/img
"""
import json, os, sys
from xml.sax.saxutils import escape

PAD, GAP = 46, 6
STEP = PAD + GAP
BG = "#232326"
FG = "#e8e8ea"
DIM = "#9a9aa2"
UNLIT = "#17171a"
PALETTE_FLAG = 0x01000000

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]


def note_name(n):
    # Ableton naming: middle C (60) is C3, so note 36 is C1.
    return f"{NAMES[n % 12]}{n // 12 - 2}"


def rgb(v):
    if v & PALETTE_FLAG:
        return "#8a8a8a"
    return "#%02x%02x%02x" % ((v >> 16) & 255, (v >> 8) & 255, v & 255)


def text_color(fill):
    r, g, b = int(fill[1:3], 16), int(fill[3:5], 16), int(fill[5:7], 16)
    return "#000" if (r * 299 + g * 587 + b * 114) / 1000 > 120 else "#fff"


def device(frame, x0=0, y0=0, pad_labels=None, top_labels=None, side_labels=None, scale=1.0, unlit=UNLIT):
    """Returns (svg group, width, height). frame: 100 colors, index = row*10+col;
    top row 91..98, right column 19..89."""
    pad_labels = pad_labels or {}
    top_labels = top_labels or {}
    side_labels = side_labels or {}
    out = []
    left, top = 14, 44
    w = left * 2 + STEP * 9 + 8
    h = top + STEP * 8 + 26
    out.append(f'<rect x="0" y="0" width="{w}" height="{h}" rx="14" fill="#101012" stroke="#3a3a40" stroke-width="2"/>')
    # top row buttons 91..98 over columns 1..8
    for i, idx in enumerate(range(91, 99)):
        x = left + i * STEP
        fill = rgb(frame[idx]) if frame[idx] else unlit
        out.append(f'<rect x="{x}" y="14" width="{PAD}" height="20" rx="10" fill="{fill}" stroke="#4a4a52"/>')
        lab = top_labels.get(idx)
        if lab:
            out.append(f'<text x="{x + PAD / 2}" y="29" font-size="9" text-anchor="middle" fill="{text_color(fill)}">{escape(lab)}</text>')
    for row in range(8, 0, -1):
        for col in range(1, 9):
            idx = row * 10 + col
            x = left + (col - 1) * STEP
            y = top + (8 - row) * STEP
            fill = rgb(frame[idx]) if frame[idx] else unlit
            out.append(f'<rect x="{x}" y="{y}" width="{PAD}" height="{PAD}" rx="7" fill="{fill}" stroke="#4a4a52"/>')
            lab = pad_labels.get((row, col))
            if lab:
                lines = lab.split("\n")
                for k, t in enumerate(lines):
                    ty = y + PAD / 2 + 4 + (k - (len(lines) - 1) / 2) * 11
                    out.append(f'<text x="{x + PAD / 2}" y="{ty}" font-size="10" text-anchor="middle" fill="{text_color(fill)}">{escape(t)}</text>')
        # right column button of this row
        idx = row * 10 + 9
        x = left + 8 * STEP
        y = top + (8 - row) * STEP + (PAD - 24) / 2
        fill = rgb(frame[idx]) if frame[idx] else unlit
        out.append(f'<rect x="{x}" y="{y}" width="20" height="24" rx="8" fill="{fill}" stroke="#4a4a52"/>')
        lab = side_labels.get(row)
        if lab:
            out.append(f'<text x="{x + 24}" y="{y + 16}" font-size="9" fill="{DIM}">{escape(lab)}</text>')
    g = f'<g transform="translate({x0},{y0}) scale({scale})">' + "".join(out) + "</g>"
    return g, w * scale, h * scale


def page(title, groups, width, height, notes=None, extra=""):
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="Helvetica, Arial, sans-serif">',
             f'<rect width="{width}" height="{height}" fill="{BG}"/>']
    parts.append(f'<text x="16" y="24" font-size="15" font-weight="bold" fill="{FG}">{escape(title)}</text>')
    parts += groups
    parts.append(extra)
    parts.append("</svg>")
    return "\n".join(parts)


def single(frame, title, caption_lines=(), pad_labels=None, top_labels=None, side_labels=None, notes_from=None, right_notes=(), outfile=None):
    labels = dict(pad_labels or {})
    if notes_from:
        for i, n in enumerate(notes_from):
            if n is not None and (i // 8 + 1, i % 8 + 1) not in labels:
                labels[(i // 8 + 1, i % 8 + 1)] = note_name(n) + "\n" + str(n)
    g, w, h = device(frame, 16, 40, labels, top_labels, side_labels)
    width = int(w + 32 + (300 if right_notes else 0))
    height = int(h + 56 + 18 * len(caption_lines))
    extra = []
    for i, line in enumerate(caption_lines):
        extra.append(f'<text x="16" y="{h + 62 + i * 18}" font-size="12" fill="{DIM}">{escape(line)}</text>')
    for i, line in enumerate(right_notes):
        extra.append(f'<text x="{w + 40}" y="{70 + i * 20}" font-size="12" fill="{FG}">{escape(line)}</text>')
    longest = max([len(l) for l in caption_lines] + [0])
    return page(title, [g], max(width, 520, int(6.3 * longest + 40)), height, extra="".join(extra))


TOP = {91: "Up", 92: "Down", 93: "Left", 94: "Right", 95: "Session", 96: "Note", 97: "Custom", 98: "Capture"}
ARROWS = {91: "▲", 92: "▼", 93: "◄", 94: "►", 95: "Session", 96: "Note", 97: "Custom", 98: "Capture"}


def write(outdir, name, svg):
    with open(os.path.join(outdir, name), "w", encoding="utf-8") as f:
        f.write(svg)
    print("wrote", name)


def blank():
    return [0] * 100


def main():
    outdir = sys.argv[1]
    os.makedirs(outdir, exist_ok=True)
    d = json.load(sys.stdin)

    # 1. The buttons
    f = blank()
    for idx in range(91, 99):
        f[idx] = 0x303038
    for row in range(1, 9):
        f[row * 10 + 9] = 0x303038
        for col in range(1, 9):
            f[row * 10 + col] = 0x202024
    f[95] = 0x502020
    f[96] = 0x20a040
    f[97] = 0x802080
    f[98] = 0xb0b0b0
    labels = {(r, c): "" for r in range(1, 9) for c in range(1, 9)}
    labels[(5, 2)] = "8 x 8\nvelocity\npads"
    g, w, h = device(f, 16, 40, labels, ARROWS, {8: "scene", 7: "scene", 1: "scene"})
    callouts = [
        "Up / Down: octave (Note Mode), track (sequencer)",
        "Left / Right: semitone (Note Mode), page (sequencer)",
        "Session: tap = Ableton Session mode, hold = Setup",
        "Note: Note Mode (press again: its settings)",
        "Custom: Custom Modes",
        "Capture MIDI: the Shift button; with Custom: sequencer",
        "Right column: 8 buttons used by the mode (Custom Modes",
        "  choose a mode, the sequencer lanes, Setup pages)",
    ]
    extra = "".join(f'<text x="{w + 36}" y="{86 + i * 22}" font-size="12" fill="{FG}">{escape(t)}</text>' for i, t in enumerate(callouts))
    write(outdir, "buttons.svg", page("The Launchpad X", [g], int(w + 520), int(h + 70), extra=extra))

    # 2. Note Mode
    nb = {91: "▲", 92: "▼", 93: "◄", 94: "►", 96: "Note"}
    write(outdir, "note-chromatic.svg", single(
        d["note_chromatic"]["frame"], "Note Mode: Chromatic layout (default, C minor, overlap 5)",
        ["Purple = C (the root), blue = other notes of C minor, unlit = notes outside the scale (they still play).",
         "Each row up adds 5 semitones: the first pad of a row is the sixth pad of the row below."],
        top_labels=nb, notes_from=d["note_chromatic"]["notes"]))
    write(outdir, "note-scale.svg", single(
        d["note_scale"]["frame"], "Note Mode: Scale layout (C natural minor)",
        ["Only notes of the scale exist, so you cannot play out of key. Bottom-left is the root."],
        top_labels=nb, notes_from=d["note_scale"]["notes"]))
    write(outdir, "note-drum.svg", single(
        d["note_drum"]["frame"], "Note Mode: Drum layout",
        ["Four blocks of 4 x 4 pads, 16 notes each. Up / Down scroll by 16 notes, Left / Right by 4."],
        top_labels=nb, notes_from=d["note_drum"]["notes"]))
    chord_labels = {}
    for r in range(1, 9):
        chord_labels[(r, 1)] = "triad"
    chord_labels[(8, 2)] = "root"
    chord_labels[(8, 3)] = "3rd+8"
    chord_labels[(8, 4)] = "5th"
    chord_labels[(8, 5)] = "7th"
    chord_labels[(8, 6)] = "5th+8"
    chord_labels[(1, 7)] = "sustain"
    chord_labels[(1, 8)] = "lock"
    for r in range(2, 9):
        chord_labels[(r, 7)] = "bank"
        chord_labels[(r, 8)] = "bank"
    write(outdir, "note-chord.svg", single(
        d["note_chord"]["frame"], "Note Mode: Chord layout",
        ["Column 1 plays triads (orange). Columns 2 to 6: one scale degree per row (the labels show the top row).",
         "Columns 7 and 8: a bank of 14 saved chords; bottom row: sustain (left) and bank lock (right)."],
        pad_labels=chord_labels, top_labels=nb))
    write(outdir, "note-chord-used.svg", single(
        d["note_chord_used"]["frame"], "Chord layout in use",
        ["A saved chord is bright white, empty slots are dim; the sustain pad is bright green when sustain is on;",
         "a pressed pad is white."],
        top_labels=nb))
    sl = {}
    for c, t in zip(range(1, 6), ["Seq", "2", "3", "4", "5"]):
        sl[(8, c)] = t
    sl[(7, 2)] = "C#"; sl[(7, 3)] = "D#"; sl[(7, 5)] = "F#"; sl[(7, 6)] = "G#"; sl[(7, 7)] = "A#"
    for c, t in zip(range(1, 8), ["C", "D", "E", "F", "G", "A", "B"]):
        sl[(6, c)] = t
    scales = ["Nat\nMinor", "Nat\nMajor", "Dorian", "Phryg", "Mixol", "Mel\nMinor", "Harm\nMinor", "Bebop\nDor",
              "Blues", "Min\nPent", "Hung\nMinor", "Ukr\nDorian", "Marva", "Todi", "Whole\nTone", "Hira-\njoshi"]
    for i, t in enumerate(scales):
        sl[(5 - i // 8, i % 8 + 1)] = t
    for i in range(16):
        sl[(3 - i // 8, i % 8 + 1)] = f"ch {i + 1}"
    for c, t in zip(range(1, 5), ["Chrom", "Scale", "Drum", "Chord"]):
        sl[(1, c)] = t
    sl[(1, 8)] = "reset"
    write(outdir, "note-settings.svg", single(
        d["note_settings"]["frame"], "Note Mode: the settings page (press Note again)",
        ["Row 8: overlap. Rows 7 and 6: root note (a piano). Rows 5 and 4: scale. Rows 3 and 2: MIDI channel.",
         "Row 1: layout (Chromatic, Scale, Drum, Chord) and, at the right, reset of octave and semitone shifts."],
        pad_labels=sl, top_labels={96: "Note"}))

    # 3. Custom Modes sheet
    titles = ["Custom 1: faders CC 7-14", "Custom 2: bipolar faders CC 15-22", "Custom 3: drum rack", "Custom 4: keyboard",
              "Custom 5: program 0-63", "Custom 6: program 64-127", "Custom 7: drum rack, lit by MIDI", "Custom 8: Programmer layout, lit by MIDI"]
    groups = []
    scale = 0.62
    wcell = 0
    for i in range(8):
        fr = d[f"custom_{i + 1}"]["frame"]
        labs = {}
        if i == 3:
            for r in range(1, 9):
                for c in range(1, 9):
                    pass
        g, w, h = device(fr, 16 + (i % 4) * 330, 56 + (i // 4) * 330, labs, None, None, scale)
        groups.append(g)
        groups.append(f'<text x="{16 + (i % 4) * 330}" y="{46 + (i // 4) * 330}" font-size="12" fill="{FG}">{escape(titles[i])}</text>')
        wcell = w
    note = "The right-hand column picks the mode: top button = Custom 1, bottom = Custom 8; the chosen one is pale green."
    write(outdir, "custom-modes.svg", page("Custom Modes (built in)", groups, 16 + 4 * 330, 56 + 2 * 330 + 10,
                                         extra=f'<text x="16" y="{56 + 2 * 330 - 6}" font-size="12" fill="{DIM}">{escape(note)}</text>'))
    cl = {(1, c): f"{c}" for c in range(1, 9)}
    cl.update({(2, c): f"{c + 8}" for c in range(1, 9)})
    write(outdir, "custom-channels.svg", single(
        d["custom_channels"]["frame"], "Custom Modes: master channel (hold Capture MIDI)",
        ["The two bottom rows are channels 1 to 8 and 9 to 16. The current channel is green; tap a pad to change it."],
        pad_labels=cl, top_labels={98: "held"}))

    # 4. Sequencer
    sq_top = {91: "Track-", 92: "Track+", 93: "Page-", 94: "Page+", 96: "Play", 97: "Custom", 98: "Shift"}
    write(outdir, "seq-drum.svg", single(
        d["seq_drum"]["frame"], "Sequencer: drum track (track 1)",
        ["Rows 8 to 5: 32 steps of the selected voice. Lower left 4 x 4: the 16 drum voices. Bottom right: page pads.",
         "Up / Down (shown as Track- / Track+) are colored like the current track."],
        top_labels=sq_top, side_labels={8: "Scale", 1: "Notes"}))
    write(outdir, "seq-melodic.svg", single(
        d["seq_melodic"]["frame"], "Sequencer: melodic track (track 2)",
        ["Rows 8 to 2: 7 values for each of 8 steps. Bottom row: page pads. Right column: lane (Notes, Velocity, Octave...)."],
        top_labels=sq_top, side_labels={1: "Notes", 2: "Velocity", 3: "Octave", 4: "Length", 5: "Random", 6: "Rate", 7: "Prob.", 8: "Scale"}))
    write(outdir, "seq-shift.svg", single(
        d["seq_shift"]["frame"], "Sequencer: settings (hold Capture MIDI = Shift)",
        ["Row 8-7: MIDI channel. 6: drum/melodic and output ports. 5: step rate. 4: clock. 3: octave/base note. 2-1: tempo."],
        top_labels=sq_top, side_labels={1: "Record", 2: "Clear", 3: "Dup.", 4: "Fixed", 5: "Mute", 6: "Solo", 7: "Projects", 8: "-"}))

    pl = {}
    for c in range(1, 9):
        pl[(8, c)] = f"load\n{c}"
        pl[(7, c)] = f"save\n{c}"
    pl[(5, 8)] = "new"
    write(outdir, "seq-projects.svg", single(
        d["seq_projects"]["frame"], "Sequencer: projects (Shift + Probability)",
        ["Top row: tap a green pad to LOAD that project (green = something is saved there, white = the one in use).",
         "Row 7: tap a pad to SAVE the current work into that slot. Pad 'new' starts an empty project.",
         "A pad flashes white when the action worked and red when it failed."],
        pad_labels=pl, top_labels=sq_top))

    write(outdir, "setup.svg", setup_page())
    write(outdir, "mode-map.svg", mode_map())


def lit(frame, idx, color):
    frame[idx] = color


def setup_page():
    panels = []
    specs = []

    f = blank()
    f[89] = 0x550000; f[79] = 0x101014; f[69] = 0x101014; f[59] = 0x101014
    f[11] = 0x4000ff; f[12] = 0x004000; f[13] = 0x401000
    for i in range(4):
        f[15 + i] = 0x6060ff if i == 0 else 0x101090
    f[25] = 0x808080
    for i in range(3):
        f[26 + i] = 0x106080
    f[95] = 0xff0000
    specs.append(("Setup page 1: mode and colors", f,
                  {(1, 1): "Perfor-\nmance", (1, 2): "Session\n(DAW)", (1, 3): "Pro-\ngrammer", (1, 5): "Nov.", (1, 6): "Mat1", (1, 7): "MXOS", (1, 8): "Lpad S",
                   (2, 5): "rainbow", (2, 6): "custom 1", (2, 7): "custom 2", (2, 8): "custom 3"}))
    f = blank()
    f[89] = 0x101014; f[79] = 0x0a0a55; f[69] = 0x101014; f[59] = 0x101014
    for i in range(8):
        f[31 + i] = 0xccccff if i == 5 else 0x101010
    f[91] = 0xffffff; f[92] = 0x202020; f[93] = 0x202020; f[94] = 0x202020
    f[51] = 0xff1010; f[52] = 0xff1010
    f[95] = 0xff0000
    labs2 = {(3, i + 1): str(i + 1) for i in range(8)}
    labs2.update({(5, 1): "Vegas", (5, 2): "sleep"})
    specs.append(("Setup page 2: brightness, rotation, Vegas mode, LED sleep", f, labs2))
    f = blank()
    f[89] = 0x101014; f[79] = 0x101014; f[69] = 0x550000; f[59] = 0x101014
    f[31] = 0x10ff10
    for i in range(3):
        f[21 + i] = 0x4000ff if i == 1 else 0x101010
    for i in range(4):
        f[51 + i] = 0xffffff if i == 0 else 0x101010
    f[95] = 0xff0000
    specs.append(("Setup page 3: velocity", f, {(3, 1): "on / off", (2, 1): "low", (2, 2): "linear", (2, 3): "high",
                                                (5, 1): "light", (5, 4): "firm"}))
    f = blank()
    f[89] = 0x101014; f[79] = 0x101014; f[69] = 0x101014; f[59] = 0x150055
    f[31] = 0x400101; f[32] = 0x10ff10; f[33] = 0x014001
    for i in range(3):
        f[21 + i] = 0x4000ff if i == 1 else 0x101010
    for i in range(3):
        f[51 + i] = 0xffffff if i == 1 else 0x101010
    f[95] = 0xff0000
    specs.append(("Setup page 4: aftertouch", f, {(3, 1): "off", (3, 2): "per\nnote", (3, 3): "channel", (2, 1): "low", (2, 2): "linear", (2, 3): "high",
                                                  (5, 1): "low", (5, 2): "mid", (5, 3): "high"}))
    f = blank()
    f[89] = 0x101014; f[79] = 0x101014; f[69] = 0x101014; f[59] = 0x101014; f[49] = 0x103030
    f[51] = 0x10ff10; f[52] = 0x10ff10
    f[95] = 0xff0000
    specs.append(("Setup page 5: MIDI clock", f, {(5, 1): "receive\nclock", (5, 2): "send\nclock"}))
    tabs = {8: "page 1", 7: "page 2", 6: "page 3", 5: "page 4", 4: "page 5"}
    groups = []
    for i, (title, fr, labs) in enumerate(specs):
        x = 16 + (i % 2) * 560
        y = 56 + (i // 2) * 500
        tl = {95: "exit"}
        if i == 1:
            tl.update({91: "default", 92: "upside\ndown", 93: "left", 94: "right"})
        g, w, h = device(fr, x, y, labs, tl, tabs, 0.95)
        groups.append(g)
        groups.append(f'<text x="{x}" y="{y - 10}" font-size="13" fill="{FG}">{escape(title)}</text>')
    return page("Setup (hold Session)", groups, 16 + 2 * 560, 56 + 3 * 500,
                extra=f'<text x="16" y="{56 + 3 * 500 - 8}" font-size="12" fill="{DIM}">The right-hand column switches between the five pages; the Session button (red) leaves Setup and keeps the settings.</text>')


def box(x, y, w, h, title, sub="", fill="#2f3a52"):
    s = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="10" fill="{fill}" stroke="#6a7aa0"/>'
    s += f'<text x="{x + w / 2}" y="{y + 24}" font-size="14" font-weight="bold" text-anchor="middle" fill="{FG}">{escape(title)}</text>'
    for i, line in enumerate(sub.split("\n")):
        s += f'<text x="{x + w / 2}" y="{y + 44 + i * 15}" font-size="11" text-anchor="middle" fill="{DIM}">{escape(line)}</text>'
    return s


def arrow(x1, y1, x2, y2, label="", dy=-6, dx=0, anchor="middle"):
    s = f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#b0b8d0" stroke-width="2" marker-end="url(#a)"/>'
    if label:
        s += f'<text x="{(x1 + x2) / 2 + dx}" y="{(y1 + y2) / 2 + dy}" font-size="11" text-anchor="{anchor}" fill="{FG}">{escape(label)}</text>'
    return s


def mode_map():
    defs = '<defs><marker id="a" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto"><path d="M0,0 L0,6 L9,3 z" fill="#b0b8d0"/></marker></defs>'
    p = [defs]
    p.append(box(40, 70, 170, 60, "Power on", "boot animation", "#3a3a3a"))
    p.append(box(300, 70, 190, 70, "Note Mode", "chromatic, scale, drum, chord", "#2f5a3a"))
    p.append(box(300, 230, 190, 70, "Custom Modes", "8 built-in modes", "#5a2f58"))
    p.append(box(300, 390, 190, 70, "Sequencer", "5 tracks, keeps playing", "#5a4a2f"))
    p.append(box(640, 70, 190, 70, "Session mode", "Ableton Live (DAW mode)", "#2f4a5a"))
    p.append(box(640, 230, 190, 70, "Setup", "hold Session in any mode\n4 pages of settings", "#5a2f2f"))
    p.append(box(640, 390, 190, 70, "Performance /\nProgrammer", "plain pads, host software", "#3a3a3a"))
    p.append(arrow(210, 100, 300, 100, "automatic"))
    p.append(arrow(395, 140, 395, 230, "Custom", dx=-34))
    p.append(arrow(405, 230, 405, 140, "Custom", dx=36))
    p.append(arrow(395, 300, 395, 390, "Capture + Custom", dx=-70))
    p.append(arrow(405, 390, 405, 300, "Custom", dx=36))
    p.append(arrow(490, 95, 640, 95, "tap Session"))
    p.append(arrow(640, 125, 490, 125, "Note", dy=14))
    p.append(arrow(640, 250, 490, 135, "Session (back)", dy=-10, dx=30))
    p.append(arrow(735, 300, 735, 390, "choose", dx=-30))
    p.append(arrow(640, 420, 490, 135, "Note", dy=24, dx=60))
    p.append('<text x="16" y="500" font-size="12" fill="#9a9aa2">Pressing Custom inside the sequencer returns to the mode you came from. Setup returns to the previous mode when you press Session.</text>')
    p.append('<text x="16" y="520" font-size="12" fill="#9a9aa2">The Note button works from Performance, Custom and Session mode; in Programmer mode the host software gets every button.</text>')
    return page("How to move between the modes", p, 880, 540)


if __name__ == "__main__":
    main()
