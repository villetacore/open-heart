from __future__ import annotations

import io
import subprocess
from collections import deque
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
CHARACTERS = ROOT / "godot/assets/sprites/characters"
PICKUPS = ROOT / "godot/assets/sprites/pickups"
EFFECTS = ROOT / "godot/assets/effects"
CELL_W = 128
CELL_H = 256
BASELINE = 245
CHARACTER_WIDTH = 100
CHARACTER_HEIGHT = 210
CHARACTER_BASELINE = 235


def alpha_bbox(image: Image.Image) -> tuple[int, int, int, int]:
    alpha = image.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        raise ValueError("empty sprite frame")
    return bbox


def keep_largest_component(image: Image.Image) -> Image.Image:
    alpha = image.getchannel("A")
    opaque = alpha.load()
    visited: set[tuple[int, int]] = set()
    components: list[list[tuple[int, int]]] = []

    for start_y in range(image.height):
        for start_x in range(image.width):
            start = (start_x, start_y)
            if start in visited or opaque[start_x, start_y] <= 8:
                continue
            queue = deque([start])
            visited.add(start)
            component: list[tuple[int, int]] = []
            while queue:
                x, y = queue.popleft()
                component.append((x, y))
                for offset_x, offset_y in (
                    (-1, -1), (0, -1), (1, -1),
                    (-1, 0), (1, 0),
                    (-1, 1), (0, 1), (1, 1),
                ):
                    next_x = x + offset_x
                    next_y = y + offset_y
                    next_pixel = (next_x, next_y)
                    if (
                        0 <= next_x < image.width
                        and 0 <= next_y < image.height
                        and next_pixel not in visited
                        and opaque[next_x, next_y] > 8
                    ):
                        visited.add(next_pixel)
                        queue.append(next_pixel)
            components.append(component)

    output = Image.new("RGBA", image.size)
    source_pixels = image.load()
    output_pixels = output.load()
    largest = max(components, key=len, default=[])
    for x, y in largest:
        output_pixels[x, y] = source_pixels[x, y]
    return output


def remove_magenta_key(image: Image.Image) -> Image.Image:
    pixels = image.load()
    for y in range(image.height):
        for x in range(image.width):
            red, green, blue, alpha = pixels[x, y]
            if alpha and red > 180 and blue > 180 and green < 95:
                pixels[x, y] = (0, 0, 0, 0)
    return image


def occupied_runs(image: Image.Image, axis: str) -> list[tuple[int, int]]:
    alpha = image.getchannel("A")
    length = image.height if axis == "y" else image.width
    occupied = []
    for position in range(length):
        crop = (
            (0, position, image.width, position + 1)
            if axis == "y"
            else (position, 0, position + 1, image.height)
        )
        occupied.append(alpha.crop(crop).getbbox() is not None)

    runs: list[tuple[int, int]] = []
    start = None
    for position, has_pixels in enumerate([*occupied, False]):
        if has_pixels and start is None:
            start = position
        elif not has_pixels and start is not None:
            runs.append((start, position))
            start = None
    return runs


def component_boxes(image: Image.Image, count: int) -> list[tuple[int, int, int, int]]:
    alpha = image.getchannel("A")
    opaque = alpha.load()
    visited: set[tuple[int, int]] = set()
    components: list[tuple[int, tuple[int, int, int, int]]] = []

    for start_y in range(image.height):
        for start_x in range(image.width):
            start = (start_x, start_y)
            if start in visited or opaque[start_x, start_y] <= 8:
                continue
            queue = deque([start])
            visited.add(start)
            size = 0
            min_x = max_x = start_x
            min_y = max_y = start_y
            while queue:
                x, y = queue.popleft()
                size += 1
                min_x = min(min_x, x)
                max_x = max(max_x, x)
                min_y = min(min_y, y)
                max_y = max(max_y, y)
                for offset_x, offset_y in (
                    (-1, -1), (0, -1), (1, -1),
                    (-1, 0), (1, 0),
                    (-1, 1), (0, 1), (1, 1),
                ):
                    next_x = x + offset_x
                    next_y = y + offset_y
                    next_pixel = (next_x, next_y)
                    if (
                        0 <= next_x < image.width
                        and 0 <= next_y < image.height
                        and next_pixel not in visited
                        and opaque[next_x, next_y] > 8
                    ):
                        visited.add(next_pixel)
                        queue.append(next_pixel)
            components.append((size, (min_x, min_y, max_x + 1, max_y + 1)))

    largest = sorted(components, reverse=True)[:count]
    return [box for _, box in largest]


def import_character_atlas(
    source_path: Path,
    names: list[str],
    source_columns: int = 8,
) -> None:
    source = remove_magenta_key(Image.open(source_path).convert("RGBA"))
    if source_columns == 8:
        row_runs = occupied_runs(source, "y")
        if len(row_runs) != len(names):
            raise ValueError(
                f"{source_path.name}: expected {len(names)} rows, got {len(row_runs)}"
            )
        row_boxes = []
        for top, bottom in row_runs:
            row_image = source.crop((0, top, source.width, bottom))
            column_runs = occupied_runs(row_image, "x")
            if len(column_runs) != 8:
                raise ValueError(
                    f"{source_path.name}: expected 8 columns, got {len(column_runs)}"
                )
            row_boxes.append(
                [(left, top, right, bottom) for left, right in column_runs]
            )
    else:
        boxes = component_boxes(source, len(names) * source_columns)
        column_boxes: list[list[tuple[int, int, int, int]]] = [
            [] for _ in range(source_columns)
        ]
        for box in boxes:
            center_x = (box[0] + box[2]) / 2
            column_index = min(
                source_columns - 1,
                int(center_x * source_columns / source.width),
            )
            column_boxes[column_index].append(box)
        row_boxes = [[] for _ in names]
        for column, frames in enumerate(column_boxes):
            if len(frames) != len(names):
                raise ValueError(
                    f"{source_path.name}/column {column}: "
                    f"expected {len(names)} frames, got {len(frames)}"
                )
            frames.sort(key=lambda box: (box[1] + box[3]) / 2)
            for row, box in enumerate(frames):
                row_boxes[row].append(box)

    for row, name in enumerate(names):
        frames = row_boxes[row]
        if source_columns == 9:
            frames = [*frames[:4], *frames[5:]]
        if len(frames) != 8:
            raise ValueError(
                f"{source_path.name}/{name}: expected 8 frames, got {len(frames)}"
            )
        frames.sort(key=lambda box: (box[0] + box[2]) / 2)
        output = Image.new("RGBA", (1024, 256))
        for column, box in enumerate(frames):
            cell = source.crop(box)
            cell = keep_largest_component(cell)
            bbox = alpha_bbox(cell)
            sprite = cell.crop(bbox)
            scale = min(108 / sprite.width, 226 / sprite.height)
            resized = sprite.resize(
                (max(1, round(sprite.width * scale)), max(1, round(sprite.height * scale))),
                Image.Resampling.NEAREST,
            )
            x = column * CELL_W + (CELL_W - resized.width) // 2
            y = BASELINE - resized.height + 1
            output.alpha_composite(resized, (x, y))
        target = CHARACTERS / name
        temporary = target.with_name(f"{target.stem}.generated.png")
        output.save(temporary)
        temporary.replace(target)
        print(f"imported {name}")


def head_image(relative_path: str) -> Image.Image:
    data = subprocess.check_output(["git", "show", f"HEAD:{relative_path}"], cwd=ROOT)
    return Image.open(io.BytesIO(data)).convert("RGBA")


def repair_character_scales() -> None:
    for path in sorted(CHARACTERS.glob("*.png")):
        current = Image.open(path).convert("RGBA")
        if current.size != (1024, 256):
            raise ValueError(f"{path.name}: expected 1024x256, got {current.size}")

        frames: list[Image.Image] = []
        for frame in range(8):
            cell = current.crop((frame * CELL_W, 0, (frame + 1) * CELL_W, CELL_H))
            cell = keep_largest_component(cell)
            bbox = alpha_bbox(cell)
            sprite = cell.crop(bbox)
            frames.append(sprite)

        output = Image.new("RGBA", (1024, 256))
        for frame, sprite in enumerate(frames):
            resized = sprite.resize(
                (CHARACTER_WIDTH, CHARACTER_HEIGHT),
                Image.Resampling.NEAREST,
            )
            x = frame * CELL_W + (CELL_W - CHARACTER_WIDTH) // 2
            y = CHARACTER_BASELINE - CHARACTER_HEIGHT + 1
            output.alpha_composite(resized, (x, y))
        temporary = path.with_name(f"{path.stem}.scaled.png")
        output.save(temporary)
        temporary.replace(path)
        print(f"scaled {path.name}: {CHARACTER_WIDTH}x{CHARACTER_HEIGHT}")


def normalize_weapon_sheets() -> None:
    weapons = ROOT / "godot/assets/sprites/weapons_fp"
    for path in sorted(weapons.glob("*.png")):
        source = Image.open(path).convert("RGBA")
        if source.width != 84 * 8:
            raise ValueError(f"{path.name}: expected 8 frames of width 84")
        output = Image.new("RGBA", source.size)
        for frame in range(8):
            cell = source.crop((frame * 84, 0, (frame + 1) * 84, source.height))
            bbox = cell.getchannel("A").getbbox()
            if bbox is None:
                continue
            sprite = cell.crop(bbox)
            scale = min(76 / sprite.width, (source.height - 6) / sprite.height)
            resized = sprite.resize(
                (max(1, round(sprite.width * scale)), max(1, round(sprite.height * scale))),
                Image.Resampling.NEAREST,
            )
            x = frame * 84 + (84 - resized.width) // 2
            y = source.height - resized.height - 3
            output.alpha_composite(resized, (x, y))
        temporary = path.with_name(f"{path.stem}.normalized.png")
        output.save(temporary)
        temporary.replace(path)
        print(f"normalized {path.relative_to(ROOT)}")


def expand_character_animations() -> None:
    idle_order = [0, 1, 2, 3, 2, 1, 0, 1]
    walk_order = [4, 5, 6, 7, 6, 5, 4, 5]
    for path in sorted(CHARACTERS.glob("*.png")):
        source = Image.open(path).convert("RGBA")
        frames = [
            source.crop((frame * CELL_W, 0, (frame + 1) * CELL_W, CELL_H))
            for frame in range(8)
        ]
        output = Image.new("RGBA", (CELL_W * 8, CELL_H * 8))

        def place(row: int, column: int, frame: Image.Image) -> None:
            output.alpha_composite(frame, (column * CELL_W, row * CELL_H))

        for column, index in enumerate(idle_order):
            place(0, column, frames[index])
        for column, index in enumerate(walk_order):
            place(1, column, frames[index])
            place(2, column, frames[index])
            place(3, column, frames[index].transpose(Image.Transpose.FLIP_LEFT_RIGHT))

        for column, index in enumerate([0, 1, 4, 5, 6, 5, 1, 0]):
            frame = frames[index]
            shifted = Image.new("RGBA", frame.size)
            shifted.alpha_composite(frame, (min(column, 7 - column) * 2, 0))
            place(4, column, shifted)

        for column, index in enumerate(idle_order):
            angle = [-2, -4, -6, -4, -2, 0, 1, 0][column]
            place(5, column, frames[index].rotate(angle, Image.Resampling.NEAREST))

        for column, index in enumerate(idle_order):
            raised = Image.new("RGBA", frames[index].size)
            raised.alpha_composite(frames[index], (0, -[0, 2, 5, 8, 5, 2, 0, 0][column]))
            place(6, column, raised)

        for column, index in enumerate(idle_order):
            height = [256, 244, 226, 202, 170, 132, 88, 48][column]
            collapsed = frames[index].resize(
                (CELL_W, height),
                Image.Resampling.NEAREST,
            )
            fallen = Image.new("RGBA", frames[index].size)
            fallen.alpha_composite(collapsed, (0, CELL_H - height))
            place(7, column, fallen)

        temporary = path.with_name(f"{path.stem}.animated.png")
        output.save(temporary)
        temporary.replace(path)
        print(f"animated {path.name}: {output.width}x{output.height}")


def remove_green_key(image: Image.Image) -> Image.Image:
    pixels = image.load()
    for y in range(image.height):
        for x in range(image.width):
            red, green, blue, alpha = pixels[x, y]
            if alpha and green > 90 and green > red * 1.10 and green > blue * 1.10:
                pixels[x, y] = (0, 0, 0, 0)
    return image


def import_static_atlas(source_path: Path) -> None:
    source = Image.open(source_path).convert("RGBA")
    targets = [
        PICKUPS / "ammo_bullets.png",
        PICKUPS / "ammo_cells.png",
        PICKUPS / "ammo_rockets.png",
        PICKUPS / "ammo_shells.png",
        PICKUPS / "grenade.png",
        PICKUPS / "heart_1up.png",
        PICKUPS / "scroll.png",
        PICKUPS / "soul.png",
        EFFECTS / "effect_blood.png",
        EFFECTS / "effect_bullet.png",
        EFFECTS / "effect_energy.png",
        EFFECTS / "effect_explosion.png",
        EFFECTS / "effect_heal.png",
        EFFECTS / "effect_mana.png",
        EFFECTS / "effect_smoke.png",
        EFFECTS / "effect_teleport.png",
    ]
    for index, target in enumerate(targets):
        column = index % 4
        row = index // 4
        left = round(column * source.width / 4)
        right = round((column + 1) * source.width / 4)
        top = round(row * source.height / 4)
        bottom = round((row + 1) * source.height / 4)
        cell = remove_green_key(source.crop((left, top, right, bottom)))
        bbox = alpha_bbox(cell)
        sprite = cell.crop(bbox)

        with Image.open(target) as original:
            width, height = original.size
        scale = min((width - 4) / sprite.width, (height - 4) / sprite.height)
        resized = sprite.resize(
            (max(1, round(sprite.width * scale)), max(1, round(sprite.height * scale))),
            Image.Resampling.NEAREST,
        )
        output = Image.new("RGBA", (width, height))
        output.alpha_composite(
            resized,
            ((width - resized.width) // 2, (height - resized.height) // 2),
        )
        temporary = target.with_name(f"{target.stem}.generated.png")
        output.save(temporary)
        temporary.replace(target)
        print(f"imported {target.relative_to(ROOT)}")


def import_weapon_master(source_path: Path) -> None:
    source = remove_green_key(Image.open(source_path).convert("RGBA"))
    names = [
        "wf_sword.png", "wf_chainsaw.png", "wf_pistol.png", "wf_shotgun.png",
        "wf_rifle.png", "wf_nailgun.png", "wf_plasma.png", "wf_rocket.png",
    ]
    weapons = ROOT / "godot/assets/sprites/weapons_fp"
    for row, name in enumerate(names):
        target = weapons / name
        with Image.open(target) as current:
            frame_h = current.height
        output = Image.new("RGBA", (84 * 8, frame_h))
        for column in range(8):
            left = round(column * source.width / 8)
            right = round((column + 1) * source.width / 8)
            top = round(row * source.height / 8)
            bottom = round((row + 1) * source.height / 8)
            cell = keep_largest_component(source.crop((left, top, right, bottom)))
            sprite = cell.crop(alpha_bbox(cell))
            scale = min(76 / sprite.width, (frame_h - 6) / sprite.height)
            resized = sprite.resize(
                (max(1, round(sprite.width * scale)), max(1, round(sprite.height * scale))),
                Image.Resampling.NEAREST,
            )
            output.alpha_composite(
                resized,
                (column * 84 + (84 - resized.width) // 2, frame_h - resized.height - 3),
            )
        temporary = target.with_name(f"{target.stem}.generated.png")
        output.save(temporary)
        temporary.replace(target)
        print(f"generated {target.relative_to(ROOT)}")


def import_item_master(source_path: Path) -> None:
    source = remove_green_key(Image.open(source_path).convert("RGBA"))
    targets = [
        CHARACTERS.parent / "items/item_medkit.png",
        CHARACTERS.parent / "items/item_potion.png",
        CHARACTERS.parent / "items/item_armor.png",
        CHARACTERS.parent / "items/item_energy_drink.png",
        CHARACTERS.parent / "items/item_gold.png",
        CHARACTERS.parent / "items/item_key.png",
        CHARACTERS.parent / "items/item_ruby.png",
        PICKUPS / "scroll.png",
        PICKUPS / "ammo_bullets.png",
        PICKUPS / "ammo_shells.png",
        PICKUPS / "ammo_cells.png",
        PICKUPS / "ammo_rockets.png",
        PICKUPS / "grenade.png",
        PICKUPS / "heart_1up.png",
        PICKUPS / "soul.png",
        PICKUPS / "quest_artifact.png",
    ]
    for index, target in enumerate(targets):
        pair = index
        row = pair // 4
        pair_column = pair % 4
        output = Image.new("RGBA", (128, 64))
        for frame in range(2):
            column = pair_column * 2 + frame
            left = round(column * source.width / 8)
            right = round((column + 1) * source.width / 8)
            top = round(row * source.height / 4)
            bottom = round((row + 1) * source.height / 4)
            cell = keep_largest_component(source.crop((left, top, right, bottom)))
            sprite = cell.crop(alpha_bbox(cell))
            scale = min(56 / sprite.width, 56 / sprite.height)
            resized = sprite.resize(
                (max(1, round(sprite.width * scale)), max(1, round(sprite.height * scale))),
                Image.Resampling.NEAREST,
            )
            output.alpha_composite(
                resized,
                (frame * 64 + (64 - resized.width) // 2, (64 - resized.height) // 2),
            )
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(f"{target.stem}.generated.png")
        output.save(temporary)
        temporary.replace(target)
        print(f"generated {target.relative_to(ROOT)}")


if __name__ == "__main__":
    generated = (
        Path(r"C:\Users\alex_pyslar\.codex\generated_images")
        / "019fa402-a01c-7362-85a2-4d4f2ad37971"
    )
    import_character_atlas(
        generated / "call_N1Yj4Lp81LnelMMGBQxmzAro.png",
        [
            "enemy_brute.png",
            "enemy_cultist.png",
            "enemy_fast.png",
            "enemy_heavy.png",
            "enemy_sniper.png",
        ],
    )
    import_character_atlas(
        generated / "call_zy8eQkcJ0dspNfA6bLdEMSHR.png",
        [
            "enemy_grunt.png",
            "npc_elena.png",
            "npc_guard.png",
            "npc_merchant.png",
            "npc_scientist.png",
        ],
    )
    import_character_atlas(
        generated / "call_Sq7517djFa89uABfMALJFlhj.png",
        ["npc_sofia.png", "npc_stranger.png", "npc_vale.png", "npc_victor.png"],
        source_columns=9,
    )
    repair_character_scales()
    expand_character_animations()
    normalize_weapon_sheets()
    import_static_atlas(
        generated / "call_kkCGUd6NC0ZcMMzFYxnq0PjC.png"
    )
    import_weapon_master(generated / "call_cGWSzUfjE99AmeREUl1jgqiX.png")
    import_item_master(generated / "call_1rm4sfQlitG5jWORkuu6Ct9p.png")
