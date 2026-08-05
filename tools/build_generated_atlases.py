from pathlib import Path
from PIL import Image


def crop_cell(image: Image.Image, column: int, row: int, columns: int, rows: int) -> Image.Image:
    left = round(column * image.width / columns)
    right = round((column + 1) * image.width / columns)
    top = round(row * image.height / rows)
    bottom = round((row + 1) * image.height / rows)
    return image.crop((left, top, right, bottom))


def build_enemy(source: Path, target: Path) -> None:
    image = Image.open(source).convert("RGBA")
    source_rows = [[crop_cell(image, column, row, 6, 4) for column in range(6)] for row in range(4)]
    atlas = Image.new("RGBA", (1024, 2048))
    rows = [
        source_rows[0],
        source_rows[1],
        list(reversed(source_rows[1])),
        source_rows[1],
        source_rows[2],
        [source_rows[3][0], source_rows[3][1]],
        source_rows[2],
        source_rows[3],
    ]
    for row_index, frames in enumerate(rows):
        for column in range(8):
            frame = frames[column % len(frames)].resize((128, 256), Image.Resampling.LANCZOS)
            atlas.alpha_composite(frame, (column * 128, row_index * 256))
    atlas.save(target)


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[1]
    build_enemy(
        root / "godot/assets/sprites/generated/enemy_blood_hound_sheet.png",
        root / "godot/assets/sprites/characters/enemy_blood_hound.png",
    )
