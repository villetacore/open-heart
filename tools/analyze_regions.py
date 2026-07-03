# -*- coding: utf-8 -*-
"""Анализ произвольной области атласа: ряды по y, затем колонки в ряду."""
from pathlib import Path
from PIL import Image
import sys

ROOT = Path(__file__).resolve().parents[1]

img = Image.open(ROOT / "godot" / "assets" / sys.argv[1]).convert("RGB")
W, H = img.size
px = img.load()

def is_bg(p, tol=26):
    return p[0] < tol and p[1] < tol and p[2] < tol

def bands(densities, start, thresh, min_gap=4, min_size=8):
    out, in_band, b0, gap = [], False, 0, 0
    for i, d in enumerate(densities):
        if d > thresh:
            if not in_band: in_band, b0 = True, i
            gap = 0
        else:
            if in_band:
                gap += 1
                if gap >= min_gap:
                    if i - gap - b0 + 1 >= min_size:
                        out.append((start + b0, start + i - gap))
                    in_band = False
    if in_band:
        out.append((start + b0, start + len(densities) - 1))
    return out

x0, y0, x1, y1 = map(int, sys.argv[2:6])
axis = sys.argv[6] if len(sys.argv) > 6 else "y"

if axis == "y":
    dens = [sum(1 for x in range(x0, x1, 2) if not is_bg(px[x, y])) for y in range(y0, y1)]
    for a, b in bands(dens, y0, thresh=4, min_gap=3):
        print(f"  y {a}..{b}  h={b-a+1}")
else:
    dens = [sum(1 for y in range(y0, y1, 2) if not is_bg(px[x, y])) for x in range(x0, x1)]
    for a, b in bands(dens, x0, thresh=1, min_gap=5):
        print(f"  x {a}..{b}  w={b-a+1}")
