# -*- coding: utf-8 -*-
"""Правочный проход после slice_atlases.py: подписи, слипшиеся ячейки, небо."""
from pathlib import Path
from PIL import Image
import sys

sys.path.insert(0, str(Path(__file__).parent))
from slice_atlases import (ROOT, ASSETS, PREV, SRC_WEAP, SRC_MAST, SRC_ENV,
                           remove_bg, bbox_pad, detect_cols, detect_rows,
                           save, contact_sheet, WEAPON_ROWS, FRAME_EDGES, CELL_W)


def alpha_rows(img, thresh=2, min_gap=2, min_size=10):
    """Полосы строк по альфе RGBA-изображения."""
    w, h = img.size
    px = img.load()
    dens = [sum(1 for x in range(0, w, 2) if px[x, y][3] > 40) for y in range(h)]
    return _bands(dens, 0, thresh, min_gap, min_size)


def alpha_cols(img, thresh=1, min_gap=3, min_size=10):
    w, h = img.size
    px = img.load()
    dens = [sum(1 for y in range(0, h, 2) if px[x, y][3] > 40) for x in range(w)]
    return _bands(dens, 0, thresh, min_gap, min_size)


def _bands(dens, start, thresh, min_gap, min_size):
    out, in_band, b0, gap = [], False, 0, 0
    for i, d in enumerate(dens):
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
        out.append((start + b0, start + len(dens) - 1))
    return out


def strip_label(img: Image.Image) -> Image.Image:
    """Убрать текстовую подпись сверху: если верхняя полоса низкая и отделена."""
    rows = alpha_rows(img, thresh=1, min_gap=3, min_size=4)
    if len(rows) >= 2 and (rows[0][1] - rows[0][0]) <= 26:
        y_cut = rows[0][1] + 2
        return img.crop((0, y_cut, img.width, img.height))
    return img


def main():
    weap = Image.open(SRC_WEAP).convert("RGB")
    mast = Image.open(SRC_MAST).convert("RGB")
    env  = Image.open(SRC_ENV).convert("RGB")

    # ── 1. Плазма без надписи «ПУШКА» ────────────────────────────────────────
    print("=== plasma fix ===")
    y0, y1 = 344, 409
    h = y1 - y0
    strip = Image.new("RGBA", (CELL_W * 8, h), (0, 0, 0, 0))
    for i in range(8):
        cell = remove_bg(weap.crop((FRAME_EDGES[i], y0, FRAME_EDGES[i+1], y1)), tol=16)
        strip.paste(cell, (i * CELL_W, 0), cell)
    save(strip, ASSETS / "sprites" / "weapons_fp" / "wf_plasma.png")

    # обновить манифест (высота кадра изменилась)
    import json
    man_p = ROOT / "godot" / "data" / "weapons_fp.json"
    man = json.loads(man_p.read_text(encoding="utf-8"))
    man["plasma"]["frame_h"] = h
    man_p.write_text(json.dumps(man, indent=2), encoding="utf-8")

    # ── 2. Патроны: один кластер вместо двух ─────────────────────────────────
    print("=== ammo fix ===")
    out_pick = ASSETS / "sprites" / "pickups"
    previews = []
    for name, box in {
        "ammo_shells":  (785, 796, 828, 876),
        "ammo_rockets": (960, 796, 1002, 876),
        "ammo_bullets": (1132, 796, 1190, 876),
        "ammo_cells":   (1320, 796, 1377, 876),
    }.items():
        img = bbox_pad(remove_bg(weap.crop(box), tol=16))
        save(img, out_pick / f"{name}.png")
        previews.append((name, img))

    # ── 3. Расходники/граната/ракета: срез подписи ───────────────────────────
    print("=== labels fix ===")
    for name, (ry0, ry1) in {
        "heart_1up": (170, 266),
        "soul":      (280, 372),
        "scroll":    (388, 478),
        "grenade":   (526, 618),
    }.items():
        cols = detect_cols(weap, (1110 if name in ("grenade",) else 1220, ry0, 1532, ry1), min_gap=8)
        if not cols:
            print(f"  [WARN] {name}")
            continue
        cx0, cx1 = cols[0]
        img = remove_bg(weap.crop((cx0 - 3, ry0, cx1 + 3, ry1)), tol=16)
        img = bbox_pad(strip_label(img))
        save(img, out_pick / f"{name}.png")
        previews.append((name, img))

    # Ракета-снаряд (предмет из ряда «РАКЕТА»)
    cols = detect_cols(weap, (1110, 630, 1532, 714), min_gap=8)
    if cols:
        cx0, cx1 = cols[0]
        img = remove_bg(weap.crop((cx0 - 3, 630, cx1 + 3, 714)), tol=16)
        img = bbox_pad(strip_label(img))
        save(img, ASSETS / "sprites" / "projectiles" / "rocket.png")
        previews.append(("rocket_proj", img))
    # удалить старые rocket_f*
    for f in (ASSETS / "sprites" / "projectiles").glob("rocket_f*.png"):
        f.unlink()

    contact_sheet(previews, PREV / "pickups.png", scale=2, cols=5)

    # ── 4. Эффекты ряд 1 (ниже заголовка) ────────────────────────────────────
    print("=== effects row1 fix ===")
    eff_dir = ASSETS / "effects"
    eprev = []
    cols = detect_cols(mast, (1000, 304, 1532, 368), min_gap=10)
    names = ["effect_explosion", "effect_bullet", "effect_energy", "effect_blood"]
    if len(cols) != 4:
        print(f"  [WARN] effects row1: {len(cols)} cols: {cols}")
    for (cx0, cx1), name in zip(cols, names):
        img = bbox_pad(remove_bg(mast.crop((cx0 - 4, 304, cx1 + 4, 368)), tol=16))
        save(img, eff_dir / f"{name}.png")
        eprev.append((name, img))
    for n in ["effect_smoke", "effect_heal", "effect_mana", "effect_teleport"]:
        eprev.append((n, Image.open(eff_dir / f"{n}.png")))
    contact_sheet(eprev, PREV / "effects.png", scale=1, cols=4)

    # ── 5. Небо: ровная сетка 4 ──────────────────────────────────────────────
    print("=== sky fix ===")
    sky_dir = ASSETS / "textures" / "sky"
    sprev = []
    sx0, sx1, sy0, sy1 = 372, 760, 894, 986
    step = (sx1 - sx0) / 4
    for i, name in enumerate(["sky_pink", "sky_purple", "sky_storm", "sky_dark"]):
        cx0 = int(sx0 + i * step) + 2
        cx1 = int(sx0 + (i + 1) * step) - 2
        img = mast.crop((cx0, sy0, cx1, sy1)).convert("RGB").resize((512, 512), Image.BICUBIC)
        save(img, sky_dir / f"{name}.png")
        sprev.append((name, img.resize((96, 96))))
    contact_sheet(sprev, PREV / "sky.png", scale=1, cols=4)

    # ── 6. Тайлы данжа: пере-нарезка начисто ─────────────────────────────────
    print("=== dungeon tiles redo ===")
    tex_dir = ASSETS / "textures" / "dungeon"
    for f in tex_dir.glob("dtile_*.png"):
        f.unlink()
    tprev, idx = [], 0

    def cut_tiles(img_src, box, thresh_r, min_gap_r):
        nonlocal idx
        rows = detect_rows(img_src, box, thresh=thresh_r, min_gap=min_gap_r, min_size=34)
        for ry0, ry1 in rows:
            cols2 = detect_cols(img_src, (box[0], ry0, box[2], ry1), min_gap=3, min_size=34)
            for cx0, cx1 in cols2:
                # почти квадратные ячейки; слипшиеся пары режем пополам
                cells = []
                wdt, hgt = cx1 - cx0, ry1 - ry0
                if wdt > hgt * 1.7:
                    n = round(wdt / hgt)
                    stp = wdt / max(n, 1)
                    for k in range(max(n, 1)):
                        cells.append((int(cx0 + k * stp), int(cx0 + (k + 1) * stp)))
                else:
                    cells.append((cx0, cx1))
                for ax0, ax1 in cells:
                    tile = img_src.crop((ax0, ry0, ax1, ry1)).convert("RGB")
                    tile = tile.resize((256, 256), Image.NEAREST)
                    save(tile, tex_dir / f"dtile_{idx:02d}.png")
                    tprev.append((f"dtile_{idx:02d}", tile.resize((72, 72))))
                    idx += 1

    env2 = Image.open(ROOT / "godot" / "assets" / "ChatGPT Image 29 июн. 2026 г., 18_08_12.png").convert("RGB")
    cut_tiles(env, (8, 872, 700, 1018), 12, 2)     # нижняя секция 18_11 (2 ряда)
    cut_tiles(env, (8, 725, 935, 866), 10, 2)      # средние ряды 18_11 (без RP-предметов)
    cut_tiles(env2, (8, 898, 1532, 1018), 10, 2)   # нижний ряд 18_08
    contact_sheet(tprev, PREV / "dungeon_tiles.png", scale=1, cols=10)

    # ── 7. Пропсы: альфа-разрезание слипшихся секций ─────────────────────────
    print("=== props redo ===")
    prop_dir = ASSETS / "sprites" / "props"
    for f in prop_dir.glob("*.png"):
        f.unlink()

    def cut_props(img_src, box, prefix, row_gap=2, col_gap=3, col_min=12):
        region = remove_bg(img_src.crop(box), tol=16)
        pprev, k = [], 0
        for ry0, ry1 in alpha_rows(region, thresh=2, min_gap=row_gap, min_size=14):
            band = region.crop((0, ry0, region.width, ry1))
            for cx0, cx1 in alpha_cols(band, thresh=1, min_gap=col_gap, min_size=col_min):
                cell = bbox_pad(band.crop((cx0, 0, cx1, band.height)))
                if cell.width < 10 or cell.height < 10:
                    continue
                save(cell, prop_dir / f"{prefix}_{k:02d}.png")
                pprev.append((f"{prefix}_{k:02d}", cell))
                k += 1
        return pprev

    pn = cut_props(env, (945, 228, 1532, 340), "neon", col_gap=4)
    ps = cut_props(env, (945, 28, 1532, 195), "street", col_gap=4)
    pf = cut_props(env, (8, 28, 565, 195), "furn", col_gap=4)
    # Ванная/декор: ванна, зеркало, унитаз — секция ВАННАЯ x570..940 y30..185
    pb = cut_props(env, (570, 28, 940, 190), "bath", col_gap=4)
    contact_sheet(pn, PREV / "props_neon.png", scale=2, cols=8)
    contact_sheet(ps, PREV / "props_street.png", scale=1, cols=10)
    contact_sheet(pf + pb, PREV / "props_furniture.png", scale=1, cols=10)

    print("\nГотово.")


if __name__ == "__main__":
    main()
