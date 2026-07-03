# -*- coding: utf-8 -*-
"""
OpenHeart — нарезка всех атласов в готовые игровые спрайты.

Источники (1536x1024, тёмный фон ~(8,11,13)):
  godot/assets/ChatGPT ... 17_47_34.png  — оружие от первого лица + патроны/расходники
  tools/ChatGPT ... 00_00_28.png         — мастер-атлас (эффекты, UI, небо, жидкости)
  godot/assets/ChatGPT ... 18_11_12.png  — окружение (тайлы, вывески, пропсы)
  godot/assets/ChatGPT ... 18_08_12.png  — доп. тайлы текстур

Выход:
  godot/assets/sprites/weapons_fp/wf_*.png   — стрипы 8 кадров, фон прозрачный
  godot/assets/sprites/pickups/*.png         — патроны/гранаты/сердца/души
  godot/assets/sprites/projectiles/*.png     — летящая ракета и т.п.
  godot/assets/effects/effect_*.png          — взрыв/кровь/дым/телепорт...
  godot/assets/ui/ui_*.png                   — иконки HUD
  godot/assets/textures/sky/*.png            — панорамы неба
  godot/assets/textures/dungeon/*.png        — тайлы стен/полов данжа
  godot/assets/sprites/props/*.png           — вывески/мебель/уличные объекты
  godot/data/weapons_fp.json                 — манифест стрипов оружия
  tools/preview/*.png                        — контрольные листы для проверки
"""
import json
from pathlib import Path
from collections import deque
from PIL import Image, ImageDraw

ROOT   = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "godot" / "assets"
DATA   = ROOT / "godot" / "data"
PREV   = ROOT / "tools" / "preview"

SRC_WEAP = ASSETS / "ChatGPT Image 29 июн. 2026 г., 17_47_34.png"
SRC_MAST = ROOT / "tools" / "ChatGPT Image 30 июн. 2026 г., 00_00_28.png"
SRC_ENV  = ASSETS / "ChatGPT Image 29 июн. 2026 г., 18_11_12.png"
SRC_ENV2 = ASSETS / "ChatGPT Image 29 июн. 2026 г., 18_08_12.png"


# ── Утилиты ──────────────────────────────────────────────────────────────────

def remove_bg(img: Image.Image, tol: int = 16) -> Image.Image:
    """Flood-fill от всех граничных пикселей, близких к фону. Жёсткий допуск
    сохраняет чёрные контуры (0,0,0) — они дальше от фона (~8,11,13), чем tol."""
    img = img.convert("RGBA")
    w, h = img.size
    px = img.load()

    border = []
    for x in range(w):
        border.append((x, 0)); border.append((x, h - 1))
    for y in range(h):
        border.append((0, y)); border.append((w - 1, y))

    # средний цвет фона по граничным пикселям (берём только тёмные)
    darks = [px[x, y] for x, y in border if px[x, y][0] < 30 and px[x, y][1] < 30 and px[x, y][2] < 30]
    if not darks:
        return img
    bg = tuple(sum(c[i] for c in darks) // len(darks) for i in range(3))

    def near_bg(p):
        return abs(p[0] - bg[0]) + abs(p[1] - bg[1]) + abs(p[2] - bg[2]) < tol

    visited = bytearray(w * h)
    q = deque((x, y) for x, y in border if near_bg(px[x, y]))
    for x, y in q:
        visited[y * w + x] = 1
    while q:
        x, y = q.popleft()
        px[x, y] = (0, 0, 0, 0)
        for nx, ny in ((x+1, y), (x-1, y), (x, y+1), (x, y-1)):
            if 0 <= nx < w and 0 <= ny < h and not visited[ny * w + nx]:
                if near_bg(px[nx, ny]):
                    visited[ny * w + nx] = 1
                    q.append((nx, ny))
    return img


def is_bg_px(p, tol=26):
    return p[0] < tol and p[1] < tol and p[2] < tol


def detect_cols(img, box, thresh=1, min_gap=5, min_size=8):
    """Вернуть [(x0,x1)] колонок контента внутри box=(x0,y0,x1,y1)."""
    x0, y0, x1, y1 = box
    px = img.load()
    dens = []
    for x in range(x0, x1):
        n = sum(1 for y in range(y0, y1, 2) if not is_bg_px(px[x, y]))
        dens.append(n)
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
                        out.append((x0 + b0, x0 + i - gap))
                    in_band = False
    if in_band:
        out.append((x0 + b0, x0 + len(dens) - 1))
    return out


def detect_rows(img, box, thresh=4, min_gap=3, min_size=8):
    x0, y0, x1, y1 = box
    px = img.load()
    dens = []
    for y in range(y0, y1):
        n = sum(1 for x in range(x0, x1, 2) if not is_bg_px(px[x, y]))
        dens.append(n)
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
                        out.append((y0 + b0, y0 + i - gap))
                    in_band = False
    if in_band:
        out.append((y0 + b0, y0 + len(dens) - 1))
    return out


def bbox_pad(img: Image.Image, pad: int = 2) -> Image.Image:
    bb = img.getbbox()
    if not bb:
        return img
    x0, y0, x1, y1 = bb
    x0 = max(0, x0 - pad); y0 = max(0, y0 - pad)
    x1 = min(img.width, x1 + pad); y1 = min(img.height, y1 + pad)
    return img.crop((x0, y0, x1, y1))


def save(img: Image.Image, path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, "PNG")
    print(f"  [OK] {path.relative_to(ROOT)}  {img.size[0]}x{img.size[1]}")


def contact_sheet(images, path, scale=2, cols=None, label=True):
    """Склеить список (name, img) в контрольный лист с подписями."""
    if not images:
        return
    cols = cols or len(images)
    cw = max(i.width for _, i in images) * scale + 8
    ch = max(i.height for _, i in images) * scale + (18 if label else 4)
    rows = (len(images) + cols - 1) // cols
    sheet = Image.new("RGBA", (cw * cols, ch * rows), (24, 20, 28, 255))
    d = ImageDraw.Draw(sheet)
    for k, (name, im) in enumerate(images):
        r, c = divmod(k, cols)
        big = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
        sheet.paste(big, (c * cw + 4, r * ch + (14 if label else 2)), big if big.mode == "RGBA" else None)
        if label:
            d.text((c * cw + 4, r * ch + 1), name, fill=(255, 160, 210, 255))
    path.parent.mkdir(parents=True, exist_ok=True)
    sheet.convert("RGB").save(path, "PNG")
    print(f"  [PREVIEW] {path.relative_to(ROOT)}")


# ── 1. Оружие от первого лица ────────────────────────────────────────────────

WEAPON_ROWS = [
    # id, y0, y1 (контентные полосы, +чуть запаса)
    ("pistol",   44, 130),
    ("shotgun",  132, 227),
    ("rifle",    230, 321),
    ("plasma",   324, 409),
    ("rocket",   412, 494),
    ("nailgun",  496, 576),
    ("chainsaw", 578, 648),
    ("sword",    655, 714),
]
FRAME_EDGES = [100, 183, 265, 348, 430, 513, 596, 678, 761]
CELL_W = 84


def slice_weapons(atlas):
    out_dir = ASSETS / "sprites" / "weapons_fp"
    manifest = {}
    previews = []
    for wid, y0, y1 in WEAPON_ROWS:
        h = y1 - y0
        strip = Image.new("RGBA", (CELL_W * 8, h), (0, 0, 0, 0))
        for i in range(8):
            cx0, cx1 = FRAME_EDGES[i], FRAME_EDGES[i + 1]
            cell = atlas.crop((cx0, y0, cx1, y1))
            cell = remove_bg(cell, tol=16)
            strip.paste(cell, (i * CELL_W, 0), cell)
        save(strip, out_dir / f"wf_{wid}.png")
        manifest[wid] = {"frames": 8, "frame_w": CELL_W, "frame_h": h,
                         "path": f"res://assets/sprites/weapons_fp/wf_{wid}.png"}
        previews.append((wid, strip))
    DATA.mkdir(parents=True, exist_ok=True)
    (DATA / "weapons_fp.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"  [OK] data/weapons_fp.json")
    contact_sheet(previews, PREV / "weapons_fp.png", scale=1, cols=1)


# ── 2. Пикапы: патроны, расходники, боевые предметы ──────────────────────────

def slice_pickups(atlas):
    out_dir = ASSETS / "sprites" / "pickups"
    previews = []

    # Патроны (idle-ряд, y 798..874)
    ammo_boxes = {
        "ammo_shells":  (783,  796, 902,  876),
        "ammo_rockets": (955,  796, 1078, 876),
        "ammo_bullets": (1128, 796, 1265, 876),
        "ammo_cells":   (1315, 796, 1455, 876),
    }
    for name, box in ammo_boxes.items():
        img = bbox_pad(remove_bg(atlas.crop(box), tol=16))
        save(img, out_dir / f"{name}.png")
        previews.append((name, img))

    # Расходники (правая колонка x1220..1530): сердце 1UP, душа, свиток.
    # Берём первый кадр (идл) каждого ряда.
    cons_rows = {
        "heart_1up": (170, 266),
        "soul":      (280, 372),
        "scroll":    (402, 478),
    }
    for name, (ry0, ry1) in cons_rows.items():
        cols = detect_cols(atlas, (1220, ry0, 1532, ry1), min_gap=8)
        if not cols:
            print(f"  [WARN] {name}: колонки не найдены")
            continue
        cx0, cx1 = cols[0]
        img = bbox_pad(remove_bg(atlas.crop((cx0 - 3, ry0, cx1 + 3, ry1)), tol=16))
        save(img, out_dir / f"{name}.png")
        previews.append((name, img))

    # Граната (первый кадр ряда y 530..618)
    cols = detect_cols(atlas, (1110, 530, 1532, 618), min_gap=8)
    if cols:
        cx0, cx1 = cols[0]
        img = bbox_pad(remove_bg(atlas.crop((cx0 - 3, 530, cx1 + 3, 618)), tol=16))
        save(img, out_dir / "grenade.png")
        previews.append(("grenade", img))

    contact_sheet(previews, PREV / "pickups.png", scale=2, cols=5)

    # Ракета-снаряд: все кадры ряда y 632..712 → projectiles/rocket_f{i}.png
    proj_dir = ASSETS / "sprites" / "projectiles"
    pprev = []
    cols = detect_cols(atlas, (1110, 630, 1532, 714), min_gap=8)
    for i, (cx0, cx1) in enumerate(cols):
        img = bbox_pad(remove_bg(atlas.crop((cx0 - 3, 630, cx1 + 3, 714)), tol=16))
        save(img, proj_dir / f"rocket_f{i}.png")
        pprev.append((f"rocket_f{i}", img))
    contact_sheet(pprev, PREV / "projectiles.png", scale=2)


# ── 3. Мастер-атлас: эффекты, UI, небо, жидкости ─────────────────────────────

def slice_master(atlas):
    previews = []

    eff_dir = ASSETS / "effects"
    eff_rows = [
        ((1000, 280, 1532, 366), ["effect_explosion", "effect_bullet", "effect_energy", "effect_blood"]),
        ((1000, 379, 1532, 450), ["effect_smoke", "effect_heal", "effect_mana", "effect_teleport"]),
    ]
    for box, names in eff_rows:
        cols = detect_cols(atlas, box, min_gap=10)
        if len(cols) != len(names):
            print(f"  [WARN] effects: найдено {len(cols)} колонок, ожидалось {len(names)}: {cols}")
        for (cx0, cx1), name in zip(cols, names):
            img = bbox_pad(remove_bg(atlas.crop((cx0 - 4, box[1], cx1 + 4, box[3])), tol=16))
            save(img, eff_dir / f"{name}.png")
            previews.append((name, img))
    contact_sheet(previews, PREV / "effects.png", scale=1, cols=4)

    # UI-иконки (y 494..556), 8 штук
    ui_dir = ASSETS / "ui"
    ui_names = ["ui_heart", "ui_ammo", "ui_key", "ui_map", "ui_save", "ui_settings", "ui_inventory", "ui_quest"]
    cols = detect_cols(atlas, (1000, 494, 1532, 558), min_gap=6)
    if len(cols) != 8:
        print(f"  [WARN] ui: найдено {len(cols)} колонок: {cols}")
    uprev = []
    for (cx0, cx1), name in zip(cols, ui_names):
        img = bbox_pad(remove_bg(atlas.crop((cx0 - 2, 494, cx1 + 2, 558)), tol=16))
        save(img, ui_dir / f"{name}.png")
        uprev.append((name, img))
    contact_sheet(uprev, PREV / "ui_icons.png", scale=2, cols=8)

    # Небо (панорамы, мягкий градиент → BICUBIC 512)
    sky_dir = ASSETS / "textures" / "sky"
    sky_names = ["sky_pink", "sky_purple", "sky_storm", "sky_dark"]
    cols = detect_cols(atlas, (368, 894, 762, 986), min_gap=5)
    sprev = []
    for (cx0, cx1), name in zip(cols, sky_names):
        img = atlas.crop((cx0, 894, cx1, 986)).convert("RGB").resize((512, 512), Image.BICUBIC)
        save(img, sky_dir / f"{name}.png")
        sprev.append((name, img.resize((96, 96))))
    contact_sheet(sprev, PREV / "sky.png", scale=1, cols=4)

    # Жидкости (лава/вода для данжа), NEAREST 256
    liq_dir = ASSETS / "textures" / "dungeon"
    liq_names = ["liquid_pink", "liquid_red", "liquid_purple", "liquid_black"]
    cols = detect_cols(atlas, (6, 894, 366, 986), min_gap=5)
    for (cx0, cx1), name in zip(cols, liq_names):
        img = atlas.crop((cx0, 894, cx1, 986)).convert("RGB").resize((256, 256), Image.NEAREST)
        save(img, liq_dir / f"{name}.png")


# ── 4. Окружение: тайлы текстур и пропсы ─────────────────────────────────────

def slice_env(atlas, atlas2):
    tex_dir = ASSETS / "textures" / "dungeon"
    previews = []

    # 18_11_12: нижняя секция "ТЕКСТУРЫ СТЕН / ПОЛОВ / ПОТОЛКОВ" (x 8..695, y 875..1015)
    rows = detect_rows(atlas, (8, 872, 700, 1018), thresh=6, min_gap=3, min_size=30)
    idx = 0
    for ry0, ry1 in rows:
        cols = detect_cols(atlas, (8, ry0, 700, ry1), min_gap=4, min_size=30)
        for cx0, cx1 in cols:
            img = atlas.crop((cx0, ry0, cx1, ry1)).convert("RGB").resize((256, 256), Image.NEAREST)
            save(img, tex_dir / f"dtile_{idx:02d}.png")
            previews.append((f"dtile_{idx:02d}", img.resize((72, 72))))
            idx += 1

    # 18_11_12: средние 2 ряда тайлов на всю ширину (y ~ 728..862)
    rows = detect_rows(atlas, (8, 725, 1532, 866), thresh=10, min_gap=3, min_size=40)
    for ry0, ry1 in rows:
        cols = detect_cols(atlas, (8, ry0, 1532, ry1), min_gap=4, min_size=40)
        for cx0, cx1 in cols:
            img = atlas.crop((cx0, ry0, cx1, ry1)).convert("RGB").resize((256, 256), Image.NEAREST)
            save(img, tex_dir / f"dtile_{idx:02d}.png")
            previews.append((f"dtile_{idx:02d}", img.resize((72, 72))))
            idx += 1

    # 18_08_12: нижний ряд тайлов (y ~ 900..1015)
    rows = detect_rows(atlas2, (8, 898, 1532, 1018), thresh=10, min_gap=3, min_size=40)
    for ry0, ry1 in rows:
        cols = detect_cols(atlas2, (8, ry0, 1532, ry1), min_gap=4, min_size=40)
        for cx0, cx1 in cols:
            img = atlas2.crop((cx0, ry0, cx1, ry1)).convert("RGB").resize((256, 256), Image.NEAREST)
            save(img, tex_dir / f"dtile_{idx:02d}.png")
            previews.append((f"dtile_{idx:02d}", img.resize((72, 72))))
            idx += 1

    contact_sheet(previews, PREV / "dungeon_tiles.png", scale=1, cols=10)

    # Пропсы: неоновые вывески (x 945..1532, y 235..335 — 2 ряда)
    prop_dir = ASSETS / "sprites" / "props"
    pprev = []
    pidx = 0
    rows = detect_rows(atlas, (945, 228, 1532, 340), thresh=4, min_gap=3, min_size=18)
    for ry0, ry1 in rows:
        cols = detect_cols(atlas, (945, ry0, 1532, ry1), min_gap=6, min_size=14)
        for cx0, cx1 in cols:
            img = bbox_pad(remove_bg(atlas.crop((cx0 - 2, ry0 - 2, cx1 + 2, ry1 + 2)), tol=16))
            save(img, prop_dir / f"neon_{pidx:02d}.png")
            pprev.append((f"neon_{pidx:02d}", img))
            pidx += 1
    contact_sheet(pprev, PREV / "props_neon.png", scale=2, cols=8)

    # Уличные объекты (x 945..1532, y 30..190)
    sprev = []
    sidx = 0
    rows = detect_rows(atlas, (945, 28, 1532, 195), thresh=4, min_gap=4, min_size=30)
    for ry0, ry1 in rows:
        cols = detect_cols(atlas, (945, ry0, 1532, ry1), min_gap=5, min_size=16)
        for cx0, cx1 in cols:
            img = bbox_pad(remove_bg(atlas.crop((cx0 - 2, ry0 - 2, cx1 + 2, ry1 + 2)), tol=16))
            save(img, prop_dir / f"street_{sidx:02d}.png")
            sprev.append((f"street_{sidx:02d}", img))
            sidx += 1
    contact_sheet(sprev, PREV / "props_street.png", scale=1, cols=10)

    # Мебель (x 8..565, y 30..190)
    fprev = []
    fidx = 0
    rows = detect_rows(atlas, (8, 28, 565, 195), thresh=4, min_gap=4, min_size=30)
    for ry0, ry1 in rows:
        cols = detect_cols(atlas, (8, ry0, 565, ry1), min_gap=5, min_size=16)
        for cx0, cx1 in cols:
            img = bbox_pad(remove_bg(atlas.crop((cx0 - 2, ry0 - 2, cx1 + 2, ry1 + 2)), tol=16))
            save(img, prop_dir / f"furn_{fidx:02d}.png")
            fprev.append((f"furn_{fidx:02d}", img))
            fidx += 1
    contact_sheet(fprev, PREV / "props_furniture.png", scale=1, cols=10)


# ── main ─────────────────────────────────────────────────────────────────────

def main():
    print("=== 1/4 Оружие FP ===")
    weap = Image.open(SRC_WEAP).convert("RGB")
    slice_weapons(weap)

    print("=== 2/4 Пикапы ===")
    slice_pickups(weap)

    print("=== 3/4 Мастер-атлас ===")
    mast = Image.open(SRC_MAST).convert("RGB")
    slice_master(mast)

    print("=== 4/4 Окружение ===")
    env  = Image.open(SRC_ENV).convert("RGB")
    env2 = Image.open(SRC_ENV2).convert("RGB")
    slice_env(env, env2)

    print("\nГотово.")


if __name__ == "__main__":
    main()
