# -*- coding: utf-8 -*-
"""Анализ атласа оружия: поиск рядов и колонок кадров по проекциям яркости."""
from pathlib import Path
from PIL import Image
import sys

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "godot" / "assets" / "ChatGPT Image 29 июн. 2026 г., 17_47_34.png"

img = Image.open(SRC).convert("RGB")
W, H = img.size
px = img.load()

def is_bg(p, tol=26):
    # фон почти чёрный (~10,10,12); текст заголовков розовый — НЕ фон
    return p[0] < tol and p[1] < tol and p[2] < tol

def row_density(x0, x1, y):
    n = 0
    for x in range(x0, x1, 2):
        if not is_bg(px[x, y]):
            n += 1
    return n

def col_density(x, y0, y1):
    n = 0
    for y in range(y0, y1, 2):
        if not is_bg(px[x, y]):
            n += 1
    return n

def bands(densities, start, thresh, min_gap=4, min_size=10):
    """Непрерывные полосы, где плотность > thresh."""
    out = []
    in_band = False
    b0 = 0
    gap = 0
    for i, d in enumerate(densities):
        if d > thresh:
            if not in_band:
                in_band = True
                b0 = i
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

mode = sys.argv[1] if len(sys.argv) > 1 else "rows"

if mode == "rows":
    # Левая панель оружия: x ≈ 8..760. Ищем ряды по y.
    x0, x1 = 8, 760
    dens = [row_density(x0, x1, y) for y in range(0, H)]
    rb = bands(dens, 0, thresh=6, min_gap=3, min_size=12)
    print("ROW BANDS (full left panel):")
    for a, b in rb:
        print(f"  y {a}..{b}  h={b-a+1}")
elif mode == "cols":
    # Колонки в конкретном ряду
    y0, y1 = int(sys.argv[2]), int(sys.argv[3])
    x0, x1 = int(sys.argv[4]) if len(sys.argv) > 4 else 8, int(sys.argv[5]) if len(sys.argv) > 5 else 762
    dens = [col_density(x, y0, y1) for x in range(x0, x1)]
    cb = bands(dens, x0, thresh=1, min_gap=5, min_size=8)
    print(f"COL BANDS y={y0}..{y1}:")
    for a, b in cb:
        print(f"  x {a}..{b}  w={b-a+1}")
