"""
Автоматическое извлечение спрайтов из ChatGPT-атласа (1536x1024).
Использует NEAREST-интерполяцию и integer scaling для чёткого pixel art.
"""

import math
from pathlib import Path
from PIL import Image, ImageEnhance
from collections import deque

SRC  = Path(r"C:\sources\OpenHeart\tools\ChatGPT Image 30 июн. 2026 г., 00_00_28.png")
ROOT = Path(r"C:\sources\OpenHeart\godot\assets")

OUT_CHARS  = ROOT / "sprites" / "characters"
OUT_ITEMS  = ROOT / "sprites" / "items"
OUT_WEAPON = ROOT / "sprites" / "weapon"
OUT_TEX    = ROOT / "textures"

FRAME_W, FRAME_H = 128, 256
ITEM_W,  ITEM_H  = 64,  64
TEX_SIZE          = 512

# ─────────────────────────────────────────────────────────────────────────────
# Section coordinates (x1, y1, x2, y2) — подтверждены pixel scanning
# ─────────────────────────────────────────────────────────────────────────────
NPC_SECTION    = (4,   18, 548, 228)   # 4 cols x 2 rows
ENEMY_SECTION  = (552, 18, 988, 228)   # 3 cols x 2 rows
WEAPON_SECTION = (992, 18, 1268, 228)  # 1 col  x 3 rows, каждая строка = 3-frame strip
ITEMS_SECTION  = (1308, 35, 1510, 228) # 2 cols x 4 rows (y=35 пропускает заголовок)
TEX_SECTION    = (4,  298, 385, 430)   # 4 cols x 2 rows (y=298 пропускает метки и заголовок)
# ─────────────────────────────────────────────────────────────────────────────


def remove_bg_floodfill(img: Image.Image, tolerance: int = 22) -> Image.Image:
    """BFS flood-fill из 4 углов, удаляет пиксели близкие к цвету фона."""
    img = img.convert("RGBA")
    w, h = img.size
    pixels = img.load()
    corners = [pixels[0, 0], pixels[w-1, 0], pixels[0, h-1], pixels[w-1, h-1]]
    bg = tuple(sum(c[i] for c in corners) // 4 for i in range(3))
    visited = bytearray(w * h)
    queue = deque([(0, 0), (w-1, 0), (0, h-1), (w-1, h-1)])
    while queue:
        x, y = queue.popleft()
        if x < 0 or y < 0 or x >= w or y >= h:
            continue
        idx = y * w + x
        if visited[idx]:
            continue
        visited[idx] = 1
        r, g, b, a = pixels[x, y]
        if (abs(r - bg[0]) + abs(g - bg[1]) + abs(b - bg[2])) < tolerance * 3:
            pixels[x, y] = (0, 0, 0, 0)
            queue.extend(((x+1, y), (x-1, y), (x, y+1), (x, y-1)))
    return img


def scale_pixel_art(img: Image.Image, target_w: int, target_h: int) -> Image.Image:
    """
    Pixel-perfect масштабирование:
    1. Обрезает до bounding box непрозрачных пикселей
    2. Находит максимальный целочисленный scale что влезает в target
    3. Масштабирует NEAREST (без мыла)
    4. Центрирует на прозрачном холсте target_w x target_h
    """
    img = img.convert("RGBA")
    bbox = img.getbbox()
    if not bbox:
        return Image.new("RGBA", (target_w, target_h), (0, 0, 0, 0))
    img = img.crop(bbox)
    iw, ih = img.size
    scale = max(min(target_w // iw, target_h // ih), 1)
    nw, nh = iw * scale, ih * scale
    scaled = img.resize((nw, nh), Image.NEAREST)
    out = Image.new("RGBA", (target_w, target_h), (0, 0, 0, 0))
    ox = (target_w - nw) // 2
    oy = (target_h - nh) // 2
    out.paste(scaled, (ox, oy), scaled)
    return out


def make_char_sheet(base: Image.Image) -> Image.Image:
    """3 кадра рядом в ячейке → 4-frame sprite sheet 512x256."""
    b = remove_bg_floodfill(base, tolerance=22)
    bw, bh = b.size
    fw = bw // 3
    frames = []
    for i in range(3):
        frame = b.crop((i * fw, 0, (i + 1) * fw, bh))
        frame = scale_pixel_art(frame, FRAME_W, FRAME_H)
        frames.append(frame)
    # 4-й кадр = зеркало 3-го (walk_1)
    frames.append(frames[2].transpose(Image.FLIP_LEFT_RIGHT))
    sheet = Image.new("RGBA", (FRAME_W * 4, FRAME_H), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        sheet.paste(f, (i * FRAME_W, 0), f)
    return sheet


def make_item_sheet(base: Image.Image) -> Image.Image:
    """Предмет → 2-frame sheet 128x64 (обычный + подсвеченный)."""
    base = remove_bg_floodfill(base, tolerance=35)
    f0 = scale_pixel_art(base, ITEM_W, ITEM_H)
    f1 = ImageEnhance.Brightness(f0).enhance(1.3)
    sheet = Image.new("RGBA", (ITEM_W * 2, ITEM_H), (0, 0, 0, 0))
    sheet.paste(f0, (0, 0), f0)
    sheet.paste(f1, (ITEM_W, 0), f1)
    return sheet


def process_texture(img: Image.Image) -> Image.Image:
    """Масштабирует thumbnail чтобы заполнить 512x512, NEAREST (чёткий pixel art)."""
    img = img.convert("RGB")
    iw, ih = img.size
    if iw == 0 or ih == 0:
        return Image.new("RGB", (TEX_SIZE, TEX_SIZE), (0, 0, 0))
    # Integer scale чтобы обе стороны >= TEX_SIZE
    scale = max(math.ceil(TEX_SIZE / iw), math.ceil(TEX_SIZE / ih), 1)
    nw, nh = iw * scale, ih * scale
    img = img.resize((nw, nh), Image.NEAREST)
    # Обрезаем центр до 512x512
    x0 = (nw - TEX_SIZE) // 2
    y0 = (nh - TEX_SIZE) // 2
    img = img.crop((x0, y0, x0 + TEX_SIZE, y0 + TEX_SIZE))
    return img


def crop_grid(img, section_box, cols, rows, names, out_dir, processor,
              margin=3, top_trim=0, bottom_trim=0):
    x1, y1, x2, y2 = section_box
    section = img.crop((x1, y1, x2, y2))
    sw, sh = section.size
    cw = sw // cols
    ch = sh // rows
    out_dir.mkdir(parents=True, exist_ok=True)
    idx = 0
    for row in range(rows):
        for col in range(cols):
            if idx >= len(names):
                break
            name = names[idx]
            idx += 1
            if not name:
                continue
            cell = section.crop((
                col * cw + margin,
                row * ch + margin + top_trim,
                (col + 1) * cw - margin,
                (row + 1) * ch - margin - bottom_trim,
            ))
            result = processor(cell)
            out_path = out_dir / f"{name}.png"
            result.save(out_path, "PNG")
            print(f"  [OK] {name} ({result.size[0]}x{result.size[1]}) -> {out_path.name}")


# ─────────────────────────────────────────────────────────────────────────────

NPC_NAMES = [
    "npc_vale", "npc_victor", "npc_elena", "npc_sofia",
    "npc_guard", "npc_merchant", "npc_scientist", "npc_stranger",
]

ENEMY_NAMES = [
    "enemy_grunt", "enemy_fast", "enemy_heavy",
    "enemy_brute", "enemy_sniper", "enemy_cultist",
]

ITEM_NAMES = [
    "item_medkit",       "item_key",
    "item_energy_drink", "item_armor",
    "item_gold",         "item_potion",
    "item_ruby",         "",
]

TEX_NAMES = [
    "wall_main",  "wall_archive", "wall_boss",    "wall_market",
    "wall_lab",   "wall_arena",   "floor_main",   "ceiling_dark",
]

WEAPON_NAMES = ["weapon_pistol", "weapon_shotgun", "weapon_magic"]


def main():
    print(f"Loading: {SRC.name}")
    atlas = Image.open(SRC)
    W, H = atlas.size
    print(f"  Size: {W}x{H}")

    dbg = SRC.parent / "debug_sections"
    dbg.mkdir(exist_ok=True)
    for name, box in [
        ("npc",    NPC_SECTION),
        ("enemy",  ENEMY_SECTION),
        ("weapon", WEAPON_SECTION),
        ("items",  ITEMS_SECTION),
        ("tex",    TEX_SECTION),
    ]:
        crop = atlas.crop(box)
        crop.save(dbg / f"sec_{name}.png")
        print(f"  debug sec_{name}: {crop.size}")

    print("\n--- NPC (4x2) ---")
    crop_grid(atlas, NPC_SECTION, cols=4, rows=2, names=NPC_NAMES,
              out_dir=OUT_CHARS, processor=make_char_sheet, top_trim=14, bottom_trim=12)

    print("\n--- Enemies (3x2) ---")
    crop_grid(atlas, ENEMY_SECTION, cols=3, rows=2, names=ENEMY_NAMES,
              out_dir=OUT_CHARS, processor=make_char_sheet, top_trim=14, bottom_trim=12)

    print("\n--- Textures (4x2) ---")
    crop_grid(atlas, TEX_SECTION, cols=4, rows=2, names=TEX_NAMES,
              out_dir=OUT_TEX, processor=process_texture)

    print("\n--- Items (2x4) ---")
    crop_grid(atlas, ITEMS_SECTION, cols=2, rows=4, names=ITEM_NAMES,
              out_dir=OUT_ITEMS, processor=make_item_sheet)

    print("\n--- Weapons (3 строки, каждая = готовый 3-frame strip) ---")
    OUT_WEAPON.mkdir(parents=True, exist_ok=True)
    wsec = atlas.crop(WEAPON_SECTION)
    ww, wh = wsec.size
    row_h = wh // 3
    for i, wname in enumerate(WEAPON_NAMES):
        t = 18 if i == 0 else 4   # пропускаем заголовок "ОРУЖИЕ" на первой строке
        row_img = wsec.crop((3, i * row_h + t, ww - 3, (i + 1) * row_h - 4))
        row_img = remove_bg_floodfill(row_img, tolerance=22)
        # Сохраняем пропорции: масштабируем по ширине (~1.4x), привязываем к низу кадра.
        # Это правильный DOOM-стиль — оружие в нижней части экрана.
        src_w, src_h = row_img.size
        scale = 384 / src_w
        new_h = max(int(src_h * scale), 1)
        scaled = row_img.resize((384, new_h), Image.NEAREST)
        sheet = Image.new("RGBA", (384, 256), (0, 0, 0, 0))
        sheet.paste(scaled, (0, 256 - new_h), scaled)
        out_p = OUT_WEAPON / f"{wname}.png"
        sheet.save(out_p, "PNG")
        print(f"  [OK] {wname} ({src_w}x{src_h} to 384x{new_h}, y={256-new_h}) -> {out_p.name}")

    print("\nAll done!")


if __name__ == "__main__":
    main()
