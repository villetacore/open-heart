"""Debug: save raw cells from NPC section to see what's in each cell."""
from pathlib import Path
from PIL import Image

SRC = Path(r"C:\sources\OpenHeart\tools\ChatGPT Image 30 июн. 2026 г., 00_00_28.png")
OUT = Path(r"C:\sources\OpenHeart\tools\debug_sections")
OUT.mkdir(exist_ok=True)

atlas = Image.open(SRC)
NPC_SECTION = (4, 18, 548, 228)
sec = atlas.crop(NPC_SECTION)
sw, sh = sec.size  # 544 x 210
print(f"NPC section: {sw}x{sh}")

cw = sw // 4
ch = sh // 2
print(f"Cell size: {cw}x{ch}")

# Save individual cells for inspection
for row in range(2):
    for col in range(4):
        cell = sec.crop((col*cw, row*ch, (col+1)*cw, (row+1)*ch))
        name = f"npc_r{row}c{col}.png"
        cell.save(OUT / name)
        print(f"  Saved {name}")

# Also save horizontal strips every 10px through the section
for y in range(0, sh, 10):
    strip = sec.crop((0, y, sw, min(y+3, sh)))
    strip.save(OUT / f"npc_strip_y{y:03d}.png")
