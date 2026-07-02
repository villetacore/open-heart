"""
Обработка конкретного загруженного спрайта:
- Убирает серый/тёмный фон через flood-fill из углов
- Создаёт 4-кадровый спрайтшит 512x256 (idle_0, idle_1, walk_0, walk_1)
- Сохраняет в characters/
"""
import sys, os
from pathlib import Path
from collections import deque
from PIL import Image, ImageEnhance, ImageFilter

SRC = r"C:\Users\alex_pyslar\AppData\Local\Temp\09780c4d-8deb-4034-9f12-7d3b523ab0f1.png"
OUT_DIR = Path(r"C:\sources\OpenHeart\godot\assets\sprites\characters")

FRAME_W, FRAME_H = 128, 256

def remove_bg_floodfill(img: Image.Image, tolerance: int = 45) -> Image.Image:
    """Flood-fill от всех 4 углов — убирает фон любого цвета."""
    img = img.convert("RGBA")
    w, h = img.size
    pixels = img.load()

    # Средний цвет фона из 8 угловых пикселей
    samples = [
        pixels[0,0], pixels[1,0], pixels[0,1],
        pixels[w-1,0], pixels[w-2,0], pixels[w-1,1],
        pixels[0,h-1], pixels[w-1,h-1],
    ]
    bg = tuple(sum(s[i] for s in samples)//len(samples) for i in range(3))

    def similar(x, y):
        r,g,b,a = pixels[x,y]
        return (abs(r-bg[0]) + abs(g-bg[1]) + abs(b-bg[2])) < tolerance * 3

    visited = bytearray(w * h)  # быстрее чем set
    queue = deque()
    starts = [(0,0),(w-1,0),(0,h-1),(w-1,h-1)]
    for sx,sy in starts:
        if not visited[sy*w+sx]:
            queue.append((sx,sy))

    while queue:
        x, y = queue.popleft()
        if x < 0 or y < 0 or x >= w or y >= h:
            continue
        idx = y * w + x
        if visited[idx]:
            continue
        visited[idx] = 1
        if similar(x, y):
            r,g,b,a = pixels[x,y]
            pixels[x,y] = (0,0,0,0)
            queue.extend(((x+1,y),(x-1,y),(x,y+1),(x,y-1)))

    return img

def pad_portrait(img: Image.Image) -> Image.Image:
    """Добавляет прозрачные поля чтобы получить соотношение 1:2."""
    iw, ih = img.size
    target_h = iw * 2
    if target_h <= ih:
        return img
    out = Image.new("RGBA", (iw, target_h), (0,0,0,0))
    # Персонаж снизу (ноги у края)
    out.paste(img, (0, target_h - ih))
    return out

def make_frames(base: Image.Image):
    """Создаёт 4 кадра: idle_0, idle_1 (зеркало), walk_0 (сдвиг), walk_1."""
    fw, fh = FRAME_W, FRAME_H
    b = base.resize((fw, fh), Image.LANCZOS)

    f0 = b.copy()                                          # idle_0 — оригинал
    f1 = b.transpose(Image.FLIP_LEFT_RIGHT)               # idle_1 — зеркало

    # walk: лёгкий вертикальный сдвиг (имитация шага)
    f2 = Image.new("RGBA", (fw, fh), (0,0,0,0))
    f2.paste(b.crop((0, 0, fw, fh - 5)), (0, 5))         # walk_0 — сдвиг вниз
    f3 = f2.transpose(Image.FLIP_LEFT_RIGHT)              # walk_1 — зеркало

    return [f0, f1, f2, f3]

def build_sheet(frames):
    sheet = Image.new("RGBA", (FRAME_W * len(frames), FRAME_H), (0,0,0,0))
    for i, f in enumerate(frames):
        sheet.paste(f, (i * FRAME_W, 0), f)
    return sheet

def main():
    print(f"Loading: {SRC}")
    img = Image.open(SRC)
    print(f"  Original size: {img.size}, mode: {img.mode}")

    print("  Removing background (flood-fill from corners)...")
    img = remove_bg_floodfill(img, tolerance=50)

    # Небольшая эрозия краёв — убирает артефакты по контуру круга
    # Создаём маску и делаем небольшой blur на краях
    r, g, b, a = img.split()
    a = a.filter(ImageFilter.MinFilter(3))  # эрозия 1px
    img = Image.merge("RGBA", (r, g, b, a))

    print("  Padding to portrait (1:2)...")
    img = pad_portrait(img)
    print(f"  After pad: {img.size}")

    print("  Building 4-frame sprite sheet...")
    frames = make_frames(img)
    sheet  = build_sheet(frames)
    print(f"  Sheet size: {sheet.size}")

    # Godot применяет set_modulate() для тинта — базовый спрайт для всех
    targets = {
        "enemy_brute":  sheet,
        "npc_stranger": sheet,
        "enemy_cultist": sheet,
    }

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for name, s in targets.items():
        out = OUT_DIR / f"{name}.png"
        s.save(out, "PNG")
        print(f"  Saved: {out}")

    print("Done!")

if __name__ == "__main__":
    main()
