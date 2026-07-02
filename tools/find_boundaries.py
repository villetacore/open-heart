"""Save annotated top-row crops to find exact section x-boundaries."""
from pathlib import Path
from PIL import Image, ImageDraw

SRC = Path(r"C:\sources\OpenHeart\tools\ChatGPT Image 30 июн. 2026 г., 00_00_28.png")
OUT = SRC.parent / "debug_sections"
OUT.mkdir(exist_ok=True)

atlas = Image.open(SRC)
W, H = atlas.size
print(f"Atlas: {W}x{H}")

# Save full top row with vertical grid lines every 50px
row = atlas.crop((0, 0, W, 230))
draw = ImageDraw.Draw(row)
for x in range(0, W, 50):
    draw.line([(x, 0), (x, 229)], fill=(255, 0, 0, 180), width=1)
    draw.text((x+2, 210), str(x), fill=(255, 0, 0))
row.save(OUT / "top_row_grid.png")
print("Saved top_row_grid.png")

# Save the header strip (y=0..20) full width to find section labels
header = atlas.crop((0, 0, W, 22))
header_big = header.resize((W, 44), Image.NEAREST)
header_big.save(OUT / "header_strip.png")
print("Saved header_strip.png")
