"""Find where texture thumbnails start in row 2."""
from pathlib import Path
from PIL import Image

SRC = Path(r"C:\sources\OpenHeart\tools\ChatGPT Image 30 июн. 2026 г., 00_00_28.png")
OUT = Path(r"C:\sources\OpenHeart\tools\debug_sections")
atlas = Image.open(SRC)

# Save horizontal slices of the texture zone (x=4-385, y=230-430)
for y in range(230, 430, 10):
    strip = atlas.crop((4, y, 385, y+8))
    strip.save(OUT / f"tex_y{y}.png")

# Also save the full texture zone
atlas.crop((4, 230, 385, 430)).save(OUT / "tex_full.png")
print("Saved tex_full.png and strips")
