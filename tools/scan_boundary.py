"""Find section boundaries in weapon+items zone."""
from pathlib import Path
from PIL import Image

SRC = Path(r"C:\sources\OpenHeart\tools\ChatGPT Image 30 июн. 2026 г., 00_00_28.png")
atlas = Image.open(SRC).convert("RGB")
y0, y1 = 40, 200

print("Scanning x=990..1536 for weapon/items boundary")
for x in range(990, 1537, 5):
    col_pixels = [atlas.getpixel((x, y)) for y in range(y0, y1)]
    dark = sum(1 for p in col_pixels if sum(p) < 60)
    mark = " <<< SEPARATOR" if dark >= 155 else ""
    print(f"{x:5d} | {dark:3d} {'|'*min(dark//3,50)}{mark}")
