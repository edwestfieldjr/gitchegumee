#!/usr/bin/env python3
"""Draw the title card: the ship head-on to the storm, as original pixel art.

Every shape is drawn here from code, in the game's 16 colors at 320x200. Nothing
is sampled from a photo. Run with no arguments to rewrite the card, or pass an
output path to draw it somewhere else:

    python3 tools/gen_title_art.py
"""

import math
import random
import re
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets/sprites/freighter/title_card.png"
W, H = 320, 200

# Keep in step with src/ui/palette.rs.
PALETTE = {
    "black": (0x00, 0x00, 0x00), "night": (0x0a, 0x16, 0x38), "deep": (0x10, 0x2c, 0x70),
    "lake": (0x00, 0x40, 0xa0), "sky": (0x50, 0x98, 0xdc), "foam": (0xc0, 0xe0, 0xf0),
    "white": (0xf0, 0xf0, 0xe8), "gray": (0xa0, 0xa0, 0xa0), "slate": (0x50, 0x50, 0x50),
    "rust": (0x80, 0x18, 0x10), "red": (0xc8, 0x30, 0x20), "amber": (0xe0, 0x80, 0x00),
    "yellow": (0xf0, 0xd0, 0x40), "sand": (0xc0, 0xa0, 0x60), "pine": (0x30, 0x70, 0x40),
    "shore": (0x18, 0x38, 0x28),
}
NAMES = list(PALETTE)
BAYER = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]

pixels = ["black"] * (W * H)
rng = random.Random(1975)


def plot(x, y, color):
    x, y = int(x), int(y)
    if 0 <= x < W and 0 <= y < H:
        pixels[y * W + x] = color


def mix(x, y, dark, light, amount):
    """Ordered dither between two colors. Amount runs from 0 (dark) to 1 (light)."""
    return light if amount * 16 > BAYER[int(y) % 4][int(x) % 4] + 0.5 else dark


def ramp(x, y, colors, amount):
    """Ordered dither along a list of colors."""
    amount = max(0.0, min(0.999, amount)) * (len(colors) - 1)
    low = int(amount)
    return mix(x, y, colors[low], colors[low + 1], amount - low)


def tilt(point):
    """The ship's pitch: her bow lifts on a sea and her stern settles."""
    x, y = point
    return x, y + (x - 62) * 0.028


def fill(points, shader):
    """Fill a polygon, asking the shader for each pixel's color."""
    ys = [p[1] for p in points]
    for y in range(max(0, math.floor(min(ys))), min(H, math.ceil(max(ys)) + 1)):
        crossings = []
        for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1]):
            if (y0 <= y + 0.5) != (y1 <= y + 0.5):
                crossings.append(x0 + (y + 0.5 - y0) * (x1 - x0) / (y1 - y0))
        crossings.sort()
        for left, right in zip(crossings[::2], crossings[1::2]):
            for x in range(max(0, math.ceil(left - 0.5)), min(W, math.floor(right - 0.5) + 1)):
                color = shader(x, y) if callable(shader) else shader
                if color:
                    plot(x, y, color)


def line(a, b, color):
    steps = max(abs(b[0] - a[0]), abs(b[1] - a[1]), 1)
    for i in range(int(steps) + 1):
        t = i / steps
        plot(round(a[0] + (b[0] - a[0]) * t), round(a[1] + (b[1] - a[1]) * t), color)


def ship_fill(points, shader):
    fill([tilt(p) for p in points], shader)


def ship_line(a, b, color):
    line(tilt(a), tilt(b), color)


def ship_dot(x, y, color, size=2):
    tx, ty = tilt((x, y))
    for dy in range(size):
        for dx in range(size):
            plot(tx + dx, ty + dy, color)


# Smooth noise for clouds and water.
_lattice = {}


def _corner(ix, iy):
    return _lattice.setdefault((ix, iy), rng.random())


def noise(x, y):
    ix, iy = math.floor(x), math.floor(y)
    fx, fy = x - ix, y - iy
    fx, fy = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy)
    top = _corner(ix, iy) + (_corner(ix + 1, iy) - _corner(ix, iy)) * fx
    low = _corner(ix, iy + 1) + (_corner(ix + 1, iy + 1) - _corner(ix, iy + 1)) * fx
    return top + (low - top) * fy


def clouds(x, y):
    total, scale, weight, weights = 0.0, 1.0, 1.0, 0.0
    for _ in range(3):
        total += noise(x * scale / 74 + 9, y * scale / 21 + 3) * weight
        weights += weight
        scale *= 2.0
        weight *= 0.42
    return total / weights


def draw_sky():
    """Heavy storm cloud, torn open low in the west so the ship stands against light."""
    colors = ["black", "night", "night", "deep", "deep", "slate", "gray"]
    for y in range(H):
        for x in range(W):
            depth = y / 150
            gap = math.exp(-(((x - 190) / 130) ** 2)) * max(0.0, depth - 0.42) * 0.85
            value = (clouds(x, y) - 0.30) * 1.0 + gap + depth * 0.06
            # Flat bands, with a narrow dithered edge between them.
            steps = value * (len(colors) - 1)
            whole = math.floor(steps)
            edge = max(0.0, (steps - whole - 0.72) / 0.28)
            plot(x, y, ramp(x, y, colors, (whole + edge) / (len(colors) - 1)))


def draw_smoke():
    """Funnel smoke torn flat by the wind."""
    for i in range(70):
        t = i / 69
        cx = 276 + t * 60 + math.sin(t * 9) * 2
        cy = 106 - t * 10 + math.sin(t * 14) * 2
        radius = 2 + t * 6
        for dy in range(-int(radius), int(radius) + 1):
            for dx in range(-int(radius), int(radius) + 1):
                if dx * dx + dy * dy <= radius * radius:
                    x, y = cx + dx, cy + dy
                    density = (1 - t * 0.75) * (0.55 + noise(x / 5, y / 5) * 0.75)
                    if density * 16 > BAYER[int(y) % 4][int(x) % 4]:
                        plot(x, y, "black" if density > 0.62 else "slate")


def draw_ship():
    # Far end first, so that the near end overlaps it.
    def stern_white(x, y):
        return ramp(x, y, ["slate", "gray", "foam"], 0.72 - (x - 205) / 170)

    # Funnel, with its band.
    ship_line((272, 88), (272, 95), "black")
    ship_fill([(263, 95), (281, 95), (281, 122), (263, 122)], "black")
    ship_fill([(264, 103), (280, 103), (280, 114), (264, 114)], lambda x, y: mix(x, y, "amber", "yellow", 0.55 - (x - 264) / 40))
    ship_line((264, 95), (264, 122), "slate")

    # After cabins.
    ship_fill([(240, 121), (306, 121), (306, 131), (240, 131)], stern_white)
    ship_line((240, 121), (306, 121), "foam")
    ship_fill([(210, 130), (316, 130), (316, 142), (210, 142)], stern_white)
    ship_line((210, 130), (316, 130), "white")
    ship_line((210, 137), (316, 137), "slate")
    for x in range(246, 304, 7):
        ship_dot(x, 125, "yellow")
    for x in range(216, 314, 8):
        ship_dot(x, 133, "amber")

    # The long hull, falling away into the dark.
    def hull_side(x, y):
        fade = (x - 62) / 255
        return ramp(x, y, ["red", "rust", "rust", "black"], 0.18 + fade * 0.72)

    ship_fill([(62, 119), (178, 130), (210, 136), (318, 142), (318, 178), (62, 178)], hull_side)
    # Hatch coamings along the deck edge, catching what light there is.
    for x in range(182, 210, 4):
        top = 130 + (x - 178) * 6 / 32
        ship_line((x, top - 1), (x + 2, top - 1), "gray")
    ship_line((178, 130), (210, 136), "slate")
    ship_line((210, 136), (318, 142), "black")
    # Plate seams and a rubbing strake.
    for x in range(92, 300, 26):
        ship_line((x, 124 + (x - 62) * 0.09), (x, 176), "black")
    ship_line((96, 152), (240, 165), "black")

    # The bow's other face, turned to the light.
    def bow_face(x, y):
        return ramp(x, y, ["rust", "red"], 0.25 + (x - 29) / 48 + (y - 119) / 220)

    ship_fill([(29, 125), (62, 119), (62, 178), (31, 178)], bow_face)
    ship_line((62, 119), (62, 178), "black")
    ship_line((63, 119), (63, 178), "red")
    # Anchor pockets.
    ship_fill([(47, 140), (55, 139), (55, 152), (47, 153)], "black")
    ship_line((47, 153), (55, 152), "rust")
    ship_fill([(77, 139), (91, 140), (91, 151), (77, 150)], "black")
    ship_line((77, 150), (91, 151), "rust")

    # The white forecastle above the red.
    def fore_side(x, y):
        return ramp(x, y, ["gray", "foam", "white"], 1.0 - (x - 62) / 150)

    def fore_face(x, y):
        return ramp(x, y, ["gray", "foam", "white"], 0.45 + (x - 28) / 42)

    ship_fill([(62, 92), (176, 113), (178, 130), (62, 119)], fore_side)
    ship_fill([(28, 104), (62, 92), (62, 119), (29, 125)], fore_face)
    ship_line((62, 92), (62, 119), "gray")
    ship_line((28, 104), (62, 92), "white")
    ship_line((62, 92), (176, 113), "white")
    ship_line((29, 125), (62, 119), "black")
    ship_line((62, 119), (178, 130), "black")
    for x, y in [(79, 108), (93, 111), (106, 113), (120, 115), (134, 117), (147, 119), (159, 121)]:
        ship_dot(x, y, "black")
    for x, y in [(35, 112), (43, 110), (52, 108)]:
        ship_dot(x, y, "black")

    # The deck house, and the rail along its wing.
    def house(x, y):
        return ramp(x, y, ["gray", "foam", "white"], 1.05 - (x - 60) / 125)

    ship_fill([(39, 85), (62, 81), (165, 96), (165, 106), (62, 95), (39, 99)], house)
    ship_line((39, 85), (62, 81), "white")
    ship_line((62, 81), (165, 96), "white")
    # Shadow under the deck house, where it meets the forecastle.
    ship_fill([(39, 99), (62, 95), (165, 106), (165, 109), (62, 98), (39, 102)], "slate")
    ship_line((39, 99), (62, 95), "black")
    ship_line((62, 95), (165, 106), "black")
    ship_line((143, 87), (165, 91), "gray")
    for x in range(143, 166, 3):
        top = 87 + (x - 143) * 4 / 22
        ship_line((x, top), (x, top + 6), "slate")
    for x, y in [(110, 95), (122, 97), (134, 98), (146, 100)]:
        ship_dot(x, y, "amber")

    # The pilothouse, with its lit windows the brightest thing in the storm.
    ship_fill([(60, 72), (119, 72), (119, 84), (60, 84)], house)
    ship_line((60, 84), (119, 91), "slate")
    ship_fill([(59, 64), (118, 64), (118, 73), (59, 73)], "black")
    for x in range(61, 117, 6):
        lit = "yellow" if (x * 7) % 5 else "amber"
        ship_fill([(x, 66), (x + 4, 66), (x + 4, 72), (x, 72)], lit)
    ship_fill([(60, 55), (117, 55), (118, 63), (58, 63)], house)
    ship_line((60, 55), (117, 55), "white")
    ship_line((56, 64), (120, 64), "black")
    ship_line((56, 63), (120, 63), "gray")

    # Rails, mast and radar.
    ship_line((60, 52), (117, 52), "slate")
    for x in range(60, 118, 8):
        ship_line((x, 52), (x, 55), "slate")
    ship_line((92, 54), (92, 21), "black")
    ship_line((93, 54), (93, 21), "slate")
    ship_line((84, 54), (91, 30), "black")
    ship_line((100, 54), (93, 30), "black")
    ship_fill([(87, 28), (98, 28), (98, 32), (87, 32)], "slate")
    ship_fill([(81, 13), (104, 13), (103, 17), (82, 17)], "slate")
    ship_line((81, 13), (104, 13), "gray")
    ship_line((92, 17), (92, 21), "black")
    # Foremast light, and the jack staff leaning out over the bow.
    ship_dot(92, 24, "white")
    ship_line((45, 60), (62, 90), "black")
    ship_line((44, 60), (61, 90), "slate")


def crest(x, base, height, period, phase):
    return base - height * (0.5 + 0.5 * math.sin(x / period + phase)) ** 2.2 + (noise(x / 13, base) - 0.5) * height * 0.5


def draw_wave(base, height, period, phase, colors, foam):
    """One rank of seas. Lower ranks are nearer, larger and darker."""
    tops = [crest(x, base, height, period, phase) for x in range(W)]
    for x in range(W):
        top = tops[x]
        for y in range(int(top), H):
            depth = (y - top) / max(height * 1.6, 1)
            streak = noise(x / 24 + phase, y / 2.2)
            plot(x, y, ramp(x, y, colors, depth * 0.9 + (streak - 0.5) * 0.22))
        rise = (base - top) / height
        plot(x, top, colors[0])
        if rise < 0.42:
            continue
        band = foam * (rise - 0.42) * 14 * (0.5 + noise(x / 6 + phase, base / 3))
        for dy in range(int(band)):
            plot(x, top + dy, "white" if dy < band * 0.45 else "foam")
        # Foam thins into lace down the face of the sea.
        for dy in range(int(band), int(band * 3.2)):
            if noise(x / 3 + phase, (top + dy) / 2) > 0.48 + dy / (band * 7 + 1):
                plot(x, top + dy, "foam" if dy < band * 2 else "sky")


def draw_sea():
    draw_wave(160, 8, 24, 0.6, ["lake", "deep", "night"], 0.5)
    draw_bow_sea()
    draw_wave(174, 15, 37, 2.6, ["lake", "deep", "night", "black"], 0.8)
    draw_wave(197, 25, 71, 3.7, ["deep", "night", "black"], 1.0)


def draw_bow_sea():
    """The sea breaking on the stem, and its spray flung up the bow."""
    for dx in range(-50, 66):
        lift = math.exp(-((dx / 17) ** 2)) * 17 + math.exp(-(((dx - 30) / 14) ** 2)) * 5
        top = 168 - lift + (noise(dx / 2.5, 4) - 0.5) * 9
        for y in range(int(top), 186):
            depth = (y - top) / max(186 - top, 1)
            lace = noise((58 + dx) / 2.5, y / 2)
            if depth < 0.25:
                color = "white"
            elif depth < 0.5:
                color = "white" if lace > 0.45 else "foam"
            elif lace > 0.32 + depth * 0.5:
                color = "foam"
            else:
                continue
            plot(58 + dx, y, color)
    for _ in range(420):
        angle = rng.uniform(-2.45, -0.75)
        reach = 24 + rng.random() ** 1.5 * 36
        x = 56 + math.cos(angle) * reach + rng.uniform(-5, 5)
        y = 158 + math.sin(angle) * reach * 0.95
        plot(x, y, rng.choice(["white", "white", "foam"]))
        if rng.random() < 0.3:
            plot(x + 1, y, "white")


def draw_snow():
    for _ in range(95):
        x, y = rng.randrange(W), rng.randrange(H)
        color = rng.choice(["white", "foam", "foam", "gray"])
        plot(x, y, color)
        if rng.random() < 0.45:
            plot(x + 1, y + 1, color)


def draw_hint_water():
    """Dark water in the corner where the game shows its pause and mute hints."""
    for y in range(166, H):
        for x in range(236, W):
            fade = min((x - 236) / 26, (y - 166) / 12) + (noise(x / 9, y / 4) - 0.5) * 0.5
            if fade * 16 > BAYER[y % 4][x % 4]:
                plot(x, y, "night")


def glyphs():
    font = (ROOT / "src/ui/font.rs").read_text()
    table = font[font.index("const GLYPHS"):font.index("];\n\n#[derive")]
    table = table[table.index("= [") + 3:]
    return [[int(v, 16) for v in row.split(",")] for row in re.findall(r"\[((?:0x[0-9A-F]{2},?){7})\]", table)]


FONT = glyphs()


def text(string, x, y, color, scale=1):
    for i, character in enumerate(string):
        for gy, row in enumerate(FONT[ord(character) - 32]):
            for gx in range(5):
                if row & (0x10 >> gx):
                    for sy in range(scale):
                        for sx in range(scale):
                            plot(x + (i * 6 + gx) * scale + sx, y + gy * scale + sy, color)


def draw_title():
    def heavy(string, y, scale):
        x = 240 - (len(string) * 6 * scale - scale) // 2
        for ox, oy in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1), (2, 2), (1, 2), (2, 1)]:
            text(string, x + ox, y + oy, "black", scale)
        text(string, x, y, "white", scale)

    def plain(string, y):
        x = 240 - (len(string) * 6 - 1) // 2
        for ox, oy in [(1, 0), (0, 1), (1, 1), (-1, 0), (0, -1)]:
            text(string, x + ox, y + oy, "black")
        text(string, x, y, "foam")

    heavy("GITCHE", 10, 4)
    heavy("GUMEE", 41, 4)
    plain("THE LAST VOYAGE OF THE", 74)
    plain("EDMUND FITZGERALD", 84)


def write_png(path):
    index = {name: i for i, name in enumerate(NAMES)}

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    rows = b"".join(b"\0" + bytes(index[c] for c in pixels[y * W:(y + 1) * W]) for y in range(H))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 3, 0, 0, 0))
        + chunk(b"PLTE", b"".join(bytes(PALETTE[name]) for name in NAMES))
        + chunk(b"IDAT", zlib.compress(rows, 9))
        + chunk(b"IEND", b"")
    )


def main():
    draw_sky()
    draw_smoke()
    draw_ship()
    draw_sea()
    draw_snow()
    draw_hint_water()
    draw_title()
    write_png(Path(sys.argv[1]) if len(sys.argv) > 1 else OUT)


if __name__ == "__main__":
    main()
