#!/usr/bin/env python3
"""Нормализация символьных спрайт-листов (512x256, 4 кадра 128x256).

AI-генерация клала фигуры разного размера и со смещением → в игре кадры
«коряво порезаны» (враг прыгает в размере, обрезается). Скрипт детектит
фигуры по альфе и переупаковывает каждую в ровный кадр: единый масштаб,
по центру по X, выровнено по низу (ноги на одной линии).

Использование:
  python normalize_chars.py preview <in.png> <out.png>   # один лист в превью
  python normalize_chars.py apply                          # все characters/ на месте
"""
import sys
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
CHARS = ROOT / "godot" / "assets" / "sprites" / "characters"

CELL_W, CELL_H, FRAMES = 128, 256, 4
MIN_W     = 18     # уже — считаем артефактом
MARGIN    = 10     # поля внутри кадра


def cell_figure(im, cx0, cx1):
    """Самая широкая фигура в столбцах [cx0,cx1) → плотно обрезанный спрайт
    (игнорирует тонкие артефакты-полоски)."""
    px = im.load(); h = im.height
    occ = [max(px[x, y][3] for y in range(h)) > 16 for x in range(cx0, cx1)]
    runs, s = [], None
    for x, v in enumerate(list(occ) + [False]):
        if v and s is None:
            s = x
        elif not v and s is not None:
            runs.append((s, x - 1)); s = None
    runs = [r for r in runs if r[1] - r[0] + 1 >= MIN_W]
    if not runs:
        return None
    a, b = max(runs, key=lambda r: r[1] - r[0])   # самая широкая
    sub = im.crop((cx0 + a, 0, cx0 + b + 1, h))
    bb = sub.getbbox()
    return sub.crop(bb) if bb else None


def normalize(im):
    """Каждую из FRAMES ячеек: взять фигуру, масштабировать в единый размер,
    центрировать по X, выровнять по низу."""
    src_cell = im.width // FRAMES
    figs = [cell_figure(im, i * src_cell, (i + 1) * src_cell) for i in range(FRAMES)]
    present = [f for f in figs if f is not None]
    if not present:
        return im.copy(), 0
    # единая высота H: максимальная, при которой самая широкая фигура ещё влезает
    # по ширине ячейки → ВСЕ кадры одного роста, центрированы, выровнены по низу
    th = CELL_H - 2 * MARGIN
    max_w = CELL_W - 2 * MARGIN
    max_aspect = max(f.width / f.height for f in present)
    target_h = min(th, max_w / max_aspect)
    out = Image.new("RGBA", (CELL_W * FRAMES, CELL_H), (0, 0, 0, 0))
    for i in range(FRAMES):
        f = figs[i] if figs[i] is not None else present[i % len(present)]
        sw, sh = f.size
        k = target_h / sh                   # все фигуры на одну высоту
        nw, nh = max(1, round(sw * k)), max(1, round(sh * k))
        sub = f.resize((nw, nh), Image.LANCZOS)
        cx = i * CELL_W + (CELL_W - nw) // 2
        cy = CELL_H - nh - MARGIN
        out.paste(sub, (cx, cy), sub)
    return out, len(present)


def main():
    if len(sys.argv) >= 4 and sys.argv[1] == "preview":
        im = Image.open(sys.argv[2]).convert("RGBA")
        out, n = normalize(im)
        out.save(sys.argv[3])
        print(f"figures={n} -> {sys.argv[3]}")
    elif len(sys.argv) >= 2 and sys.argv[1] == "apply":
        for p in sorted(CHARS.glob("*.png")):
            im = Image.open(p).convert("RGBA")
            out, n = normalize(im)
            out.save(p)
            print(f"{p.name}: figures={n}")
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
