# -*- coding: utf-8 -*-
"""Финальная нарезка пропсов по измеренным боксам (18_11_12.png)."""
from pathlib import Path
from PIL import Image
import sys

sys.path.insert(0, str(Path(__file__).parent))
from slice_atlases import ROOT, ASSETS, PREV, SRC_ENV, remove_bg, bbox_pad, save, contact_sheet

# имя → (x0, y0, x1, y1) в координатах атласа
PROPS = {
    # Неоновые вывески, ряд 1 (y 230..292)
    "neon_femboy":       (947, 230, 1040, 292),
    "neon_good_boy":     (1040, 230, 1095, 292),
    "neon_traps":        (1105, 230, 1192, 292),
    "neon_heart":        (1195, 230, 1262, 292),
    "neon_boys":         (1268, 230, 1348, 292),
    "neon_femboy_club":  (1352, 230, 1452, 292),
    "neon_catface":      (1460, 230, 1512, 292),
    # Ряд 2 (y 293..348)
    "neon_kawaii":       (950, 293, 1062, 348),
    "neon_game_over":    (1073, 293, 1130, 348),
    "neon_trans_rights": (1140, 293, 1255, 348),
    "neon_love_wins":    (1258, 293, 1325, 348),
    "neon_uwu":          (1330, 293, 1390, 348),
    "neon_pills":        (1398, 293, 1453, 348),
    "neon_hearts":       (1456, 293, 1518, 348),
    # Уличные объекты
    "street_dumpster":   (948, 30, 1092, 165),
    "street_bags":       (1098, 95, 1185, 165),
    "street_cone":       (1183, 55, 1226, 148),
    "street_trashcan":   (1213, 28, 1275, 122),
    "street_bench":      (1076, 152, 1178, 200),
    "street_fountain":   (1193, 148, 1250, 200),
    "street_vending":    (1303, 148, 1360, 200),
    "street_phone":      (1395, 38, 1437, 158),
    "street_fan_unit":   (1335, 28, 1402, 112),
    "street_grate_table":(945, 152, 1055, 200),
    # Мебель
    "furn_bed":          (8, 42, 162, 172),
    "furn_dresser":      (140, 58, 233, 136),
    "furn_wardrobe":     (243, 53, 307, 172),
    "furn_mirror":       (322, 55, 377, 172),
    "furn_chair":        (375, 53, 442, 167),
    "furn_stool":        (433, 120, 477, 167),
    "furn_desk":         (440, 53, 545, 167),
    # Ванная
    "bath_tub":          (570, 32, 667, 182),
    "bath_shower":       (675, 32, 772, 152),
    "bath_sink":         (778, 32, 852, 167),
    "bath_toilet":       (855, 32, 928, 172),
}

def main():
    env = Image.open(SRC_ENV).convert("RGB")
    prop_dir = ASSETS / "sprites" / "props"
    # подчистить прошлые версии
    for f in prop_dir.glob("*.png"):
        f.unlink()
    prev = []
    for name, box in PROPS.items():
        img = bbox_pad(remove_bg(env.crop(box), tol=16))
        save(img, prop_dir / f"{name}.png")
        prev.append((name, img))
    contact_sheet(prev, PREV / "props_final.png", scale=1, cols=7)

if __name__ == "__main__":
    main()
