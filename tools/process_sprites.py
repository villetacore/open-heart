#!/usr/bin/env python3
"""
OpenHeart Asset Processor
Форматирует сгенерированные изображения для использования в игре.

Установка: pip install Pillow

Использование:
  python process_sprites.py character <input.png> <npc_id|enemy_id>
      Пример: python process_sprites.py character vale_raw.png npc_vale
      Пример: python process_sprites.py character grunt_raw.png enemy_grunt

  python process_sprites.py texture <input.png> <name>
      Пример: python process_sprites.py texture brick_raw.png wall_main

  python process_sprites.py item <input.png> <item_id>
      Пример: python process_sprites.py item medkit_raw.png item_medkit

  python process_sprites.py weapon <input.png>
      Пример: python process_sprites.py weapon gun_raw.png

  python process_sprites.py batch_chars <folder/>
      Обрабатывает все PNG в папке как character-спрайты (имя файла = id)

  python process_sprites.py sheet <frame0.png> [frame1.png ...] <output.png>
      Склеивает несколько кадров в горизонтальный спрайтшит

Форматы вывода:
  character  -> godot/assets/sprites/characters/<id>.png   (512x256, 4 кадра 128x256)
  texture    -> godot/assets/textures/<name>.png           (512x512, тайлинг)
  item       -> godot/assets/sprites/items/<id>.png        (128x64, 2 кадра 64x64)
  weapon     -> godot/assets/sprites/weapon/weapon_pistol.png (384x256, 3 кадра 128x256)
"""

import sys
import os
from pathlib import Path
from PIL import Image, ImageFilter, ImageEnhance

# ── Пути ─────────────────────────────────────────────────────────────────────

SCRIPT_DIR   = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
ASSETS       = PROJECT_ROOT / "godot" / "assets"
OUT_CHARS    = ASSETS / "sprites" / "characters"
OUT_ITEMS    = ASSETS / "sprites" / "items"
OUT_WEAPON   = ASSETS / "sprites" / "weapon"
OUT_TEXTURES = ASSETS / "textures"

# ── Форматы ───────────────────────────────────────────────────────────────────

CHAR_FRAME_W  = 128
CHAR_FRAME_H  = 256
CHAR_FRAMES   = 4          # idle_0 | idle_1 | walk_0 | walk_1

ITEM_FRAME_W  = 64
ITEM_FRAME_H  = 64
ITEM_FRAMES   = 2

WEAPON_FRAME_W = 128
WEAPON_FRAME_H = 256
WEAPON_FRAMES  = 3

TEX_SIZE = 512

# ── Утилиты ───────────────────────────────────────────────────────────────────

def ensure_rgba(img: Image.Image) -> Image.Image:
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    return img

def remove_white_bg(img: Image.Image, threshold: int = 240) -> Image.Image:
    """Убирает белый/светлый фон, делая его прозрачным."""
    img = ensure_rgba(img)
    data = img.load()
    w, h = img.size
    for y in range(h):
        for x in range(w):
            r, g, b, a = data[x, y]
            if r > threshold and g > threshold and b > threshold:
                data[x, y] = (r, g, b, 0)
    return img

def remove_black_bg(img: Image.Image, threshold: int = 20) -> Image.Image:
    """Убирает чёрный фон."""
    img = ensure_rgba(img)
    data = img.load()
    w, h = img.size
    for y in range(h):
        for x in range(w):
            r, g, b, a = data[x, y]
            if r < threshold and g < threshold and b < threshold:
                data[x, y] = (r, g, b, 0)
    return img

def center_crop_to_ratio(img: Image.Image, target_w: int, target_h: int) -> Image.Image:
    """Центрированная обрезка до нужного соотношения сторон."""
    iw, ih = img.size
    target_ratio = target_w / target_h
    current_ratio = iw / ih
    if current_ratio > target_ratio:
        new_w = int(ih * target_ratio)
        x0 = (iw - new_w) // 2
        img = img.crop((x0, 0, x0 + new_w, ih))
    elif current_ratio < target_ratio:
        new_h = int(iw / target_ratio)
        y0 = (ih - new_h) // 2
        img = img.crop((0, y0, iw, y0 + new_h))
    return img

def pad_to_ratio(img: Image.Image, target_w: int, target_h: int) -> Image.Image:
    """Добавляет прозрачные поля до нужного соотношения (не обрезает)."""
    iw, ih = img.size
    target_ratio = target_w / target_h
    current_ratio = iw / ih
    if abs(current_ratio - target_ratio) < 0.01:
        return img
    if current_ratio > target_ratio:
        new_h = int(iw / target_ratio)
        out = Image.new("RGBA", (iw, new_h), (0, 0, 0, 0))
        out.paste(img, (0, (new_h - ih) // 2))
    else:
        new_w = int(ih * target_ratio)
        out = Image.new("RGBA", (new_w, ih), (0, 0, 0, 0))
        out.paste(img, ((new_w - iw) // 2, 0))
    return out

def make_sprite_sheet(frames: list[Image.Image], frame_w: int, frame_h: int) -> Image.Image:
    """Склеивает кадры в горизонтальный спрайтшит."""
    resized = [f.resize((frame_w, frame_h), Image.LANCZOS) for f in frames]
    sheet = Image.new("RGBA", (frame_w * len(resized), frame_h), (0, 0, 0, 0))
    for i, frame in enumerate(resized):
        sheet.paste(frame, (i * frame_w, 0))
    return sheet

def auto_remove_bg(img: Image.Image) -> Image.Image:
    """Автоматически убирает фон (пробует белый, потом чёрный)."""
    rgba = ensure_rgba(img)
    # Проверяем угловые пиксели для определения типа фона
    corners = [rgba.getpixel((0, 0)), rgba.getpixel((rgba.width-1, 0)),
               rgba.getpixel((0, rgba.height-1)), rgba.getpixel((rgba.width-1, rgba.height-1))]
    avg_r = sum(c[0] for c in corners) / 4
    avg_g = sum(c[1] for c in corners) / 4
    avg_b = sum(c[2] for c in corners) / 4
    if avg_r > 200 and avg_g > 200 and avg_b > 200:
        return remove_white_bg(rgba, threshold=230)
    elif avg_r < 30 and avg_g < 30 and avg_b < 30:
        return remove_black_bg(rgba, threshold=25)
    # Иначе — пробуем убрать белый
    return remove_white_bg(rgba, threshold=240)

# ── Команды ───────────────────────────────────────────────────────────────────

def process_character(input_path: str, char_id: str):
    """
    Преобразует изображение персонажа/врага в спрайтшит 512x256 (4 кадра 128x256).
    Из одного изображения создаёт 4 кадра: idle_0, idle_1 (зеркало), walk_0 (слегка наклон), walk_1.
    """
    img = Image.open(input_path)
    img = auto_remove_bg(img)

    # Приводим к соотношению 1:2 (ширина:высота)
    img = pad_to_ratio(img, 1, 2)
    img = img.resize((CHAR_FRAME_W, CHAR_FRAME_H), Image.LANCZOS)

    # Кадр 0: исходный
    f0 = img.copy()
    # Кадр 1: горизонтальное зеркало (idle swaying)
    f1 = img.transpose(Image.FLIP_LEFT_RIGHT)
    # Кадр 2: walk — лёгкий сдвиг вниз (имитация шага)
    f2 = Image.new("RGBA", (CHAR_FRAME_W, CHAR_FRAME_H), (0, 0, 0, 0))
    f2.paste(img.crop((0, 0, CHAR_FRAME_W, CHAR_FRAME_H - 4)),
             (0, 4))
    # Кадр 3: walk + зеркало
    f3 = f2.transpose(Image.FLIP_LEFT_RIGHT)

    sheet = make_sprite_sheet([f0, f1, f2, f3], CHAR_FRAME_W, CHAR_FRAME_H)

    out_path = OUT_CHARS / f"{char_id}.png"
    OUT_CHARS.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, "PNG")
    print(f"[OK] character -> {out_path}  ({sheet.width}x{sheet.height})")

def process_character_frames(frame_paths: list[str], char_id: str):
    """Создаёт спрайтшит из 2-4 отдельных кадров."""
    frames = []
    for p in frame_paths[:4]:
        img = Image.open(p)
        img = auto_remove_bg(img)
        img = pad_to_ratio(img, 1, 2)
        frames.append(img)
    # Если кадров < 4, дополняем зеркалами
    while len(frames) < 4:
        frames.append(frames[-1].transpose(Image.FLIP_LEFT_RIGHT))
    sheet = make_sprite_sheet(frames, CHAR_FRAME_W, CHAR_FRAME_H)
    out_path = OUT_CHARS / f"{char_id}.png"
    OUT_CHARS.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, "PNG")
    print(f"[OK] character (multi-frame) -> {out_path}")

def process_texture(input_path: str, tex_name: str):
    """
    Преобразует изображение в тайловую текстуру 512x512.
    Опционально делает края чуть мягче для бесшовного тайлинга.
    """
    img = Image.open(input_path).convert("RGB")
    img = img.resize((TEX_SIZE, TEX_SIZE), Image.LANCZOS)

    # Мягкое усиление контраста (текстуры часто выглядят блекло)
    img = ImageEnhance.Contrast(img).enhance(1.15)
    img = ImageEnhance.Sharpness(img).enhance(1.2)

    out_path = OUT_TEXTURES / f"{tex_name}.png"
    OUT_TEXTURES.mkdir(parents=True, exist_ok=True)
    img.save(out_path, "PNG")
    print(f"[OK] texture -> {out_path}  ({img.width}x{img.height})")

def process_item(input_path: str, item_id: str):
    """
    Преобразует иконку предмета в спрайтшит 128x64 (2 кадра 64x64).
    Кадр 0: исходный, кадр 1: чуть ярче (анимация свечения).
    """
    img = Image.open(input_path)
    img = auto_remove_bg(img)
    img = img.resize((ITEM_FRAME_W, ITEM_FRAME_H), Image.LANCZOS)

    f0 = img.copy()
    f1 = ImageEnhance.Brightness(img).enhance(1.3)  # «мерцание»

    sheet = make_sprite_sheet([f0, f1], ITEM_FRAME_W, ITEM_FRAME_H)
    out_path = OUT_ITEMS / f"{item_id}.png"
    OUT_ITEMS.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, "PNG")
    print(f"[OK] item -> {out_path}  ({sheet.width}x{sheet.height})")

def process_weapon(input_path: str):
    """
    Преобразует спрайт оружия в HUD-лист 384x256 (3 кадра 128x256).
    Кадр 0: покой, кадр 1: отдача вверх, кадр 2: отдача вниз.
    """
    img = Image.open(input_path)
    img = auto_remove_bg(img)
    img = pad_to_ratio(img, 1, 2)
    img = img.resize((WEAPON_FRAME_W, WEAPON_FRAME_H), Image.LANCZOS)

    f0 = img.copy()

    # Кадр 1: сдвинут вверх (отдача вверх)
    f1 = Image.new("RGBA", (WEAPON_FRAME_W, WEAPON_FRAME_H), (0, 0, 0, 0))
    f1.paste(img.crop((0, 8, WEAPON_FRAME_W, WEAPON_FRAME_H)), (0, 0))

    # Кадр 2: сдвинут вниз (возврат)
    f2 = Image.new("RGBA", (WEAPON_FRAME_W, WEAPON_FRAME_H), (0, 0, 0, 0))
    f2.paste(img.crop((0, 0, WEAPON_FRAME_W, WEAPON_FRAME_H - 8)), (0, 8))

    sheet = make_sprite_sheet([f0, f1, f2], WEAPON_FRAME_W, WEAPON_FRAME_H)
    out_path = OUT_WEAPON / "weapon_pistol.png"
    OUT_WEAPON.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, "PNG")
    print(f"[OK] weapon -> {out_path}  ({sheet.width}x{sheet.height})")

def process_sheet(frame_paths: list[str], output_path: str):
    """Склеивает произвольные кадры в горизонтальный лист (выбирает размер по первому)."""
    if not frame_paths:
        print("Нет входных файлов!"); return
    base = Image.open(frame_paths[0])
    fw, fh = base.size
    frames = [Image.open(p).resize((fw, fh), Image.LANCZOS) for p in frame_paths]
    sheet  = Image.new("RGBA", (fw * len(frames), fh), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        sheet.paste(ensure_rgba(f), (i * fw, 0))
    sheet.save(output_path, "PNG")
    print(f"[OK] sheet -> {output_path}  ({sheet.width}x{sheet.height})")

def batch_chars(folder: str):
    """Обрабатывает все PNG в папке как character-спрайты."""
    folder_path = Path(folder)
    pngs = list(folder_path.glob("*.png"))
    if not pngs:
        print(f"PNG не найдены в {folder}"); return
    for p in pngs:
        char_id = p.stem
        try:
            process_character(str(p), char_id)
        except Exception as e:
            print(f"[ERR] {p.name}: {e}")

# ── Точка входа ───────────────────────────────────────────────────────────────

def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__); return

    cmd = args[0]
    if cmd == "character":
        if len(args) < 3:
            print("Использование: character <input.png> <char_id>"); return
        process_character(args[1], args[2])

    elif cmd == "character_frames":
        # character_frames frame0.png frame1.png ... char_id
        if len(args) < 3:
            print("Использование: character_frames <frame0.png> [...] <char_id>"); return
        process_character_frames(args[1:-1], args[-1])

    elif cmd == "texture":
        if len(args) < 3:
            print("Использование: texture <input.png> <name>"); return
        process_texture(args[1], args[2])

    elif cmd == "item":
        if len(args) < 3:
            print("Использование: item <input.png> <item_id>"); return
        process_item(args[1], args[2])

    elif cmd == "weapon":
        if len(args) < 2:
            print("Использование: weapon <input.png>"); return
        process_weapon(args[1])

    elif cmd == "sheet":
        if len(args) < 3:
            print("Использование: sheet <frame0.png> [...] <output.png>"); return
        process_sheet(args[1:-1], args[-1])

    elif cmd == "batch_chars":
        if len(args) < 2:
            print("Использование: batch_chars <folder/>"); return
        batch_chars(args[1])

    else:
        print(f"Неизвестная команда: {cmd}")
        print(__doc__)

if __name__ == "__main__":
    main()
