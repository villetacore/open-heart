"""Сборка карты хаба «Неоновый квартал» из плана города.

Хаб — единственная карта, которую игрок видит между забегами, и руками
полторы тысячи строк JSON не поддержать: кварталы разъезжаются, дома встают
на NPC, улицы упираются в стены. Поэтому карта собирается отсюда — по плану
улиц, а не по списку координат.

    python tools/build_hub_map.py            # пересобрать game/presets/core/maps/hub.json
    python tools/build_hub_map.py --check    # только проверить, что файл совпадает
    python tools/build_hub_map.py --plan out.png   # верхний вид для глазами-проверки

Результат детерминирован: одно и то же зерно даёт байт в байт тот же файл,
иначе `content_hash` пресета менялся бы на ровном месте.

Что план гарантирует:

* дома не встают на NPC, точки спавна врагов, площадь, врата и улицы;
* фасады смотрят на улицу, вывески и неоновые полосы — только с этой стороны;
* точечных источников света не больше шести на плитку земли 25×25 м
  (мобильный рендер Godot держит ограниченное число ламп на объект).

Правила формата карты — `core/src/worldgen/map_def.rs` и docs/DATA_FORMATS.md.
"""

from __future__ import annotations

import argparse
import io
import json
import math
import os
import random
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PRESET = os.path.join(ROOT, "game", "presets", "core")
MAP_PATH = os.path.join(PRESET, "maps", "hub.json")
NPCS_PATH = os.path.join(PRESET, "npcs.json")

SEED = 20260827

# ── План города ──────────────────────────────────────────────────────────────

GROUND = 200.0
# Кварталы застройки живут внутри этого квадрата; дальше — стена-граница.
CITY = 86.0
# Шаг сетки участков. 8 м — это дом в 6 м с двухметровым переулком.
CELL = 8.0
GAP = 2.0

PLAYER_SPAWN = [0.0, 1.1, 12.0]
GATE = [0.0, 2.6, -58.0]

# Районы: центр, радиус, палитра и профиль застройки. Идентификаторы и имена
# оставлены прежними — на них смотрит HUD и на них ссылается лор.
DISTRICTS = [
    {
        "id": "heart_plaza",
        "name_ru": "Площадь Сердца",
        "name_en": "Heart Plaza",
        "center": [0.0, 0.0],
        "radius": 18.0,
        "color": [1.0, 0.32, 0.72],
        "outline_tex": "liquid_pink",
        "tex": "wall_main",
        "heights": (10.0, 19.0),
        "landmark": {"tex": "neon_heart", "pos": [0.0, 1.0, 8.0], "px": 0.026},
        "signs": ["neon_heart", "neon_hearts", "neon_love_wins", "neon_uwu"],
    },
    {
        "id": "night_market",
        "name_ru": "Ночной рынок",
        "name_en": "Night Market",
        "center": [-34.0, 14.0],
        "radius": 23.0,
        "color": [1.0, 0.48, 0.2],
        "outline_tex": "liquid_red",
        "tex": "wall_market",
        "heights": (6.0, 13.0),
        "landmark": {"tex": "neon_femboy_club", "pos": [-34.0, 1.2, 18.0], "px": 0.024},
        "signs": ["neon_kawaii", "neon_pills", "neon_femboy", "neon_traps", "neon_uwu"],
    },
    {
        "id": "mercy_ward",
        "name_ru": "Квартал Милосердия",
        "name_en": "Mercy Ward",
        "center": [36.0, 14.0],
        "radius": 23.0,
        "color": [0.25, 0.85, 1.0],
        "outline_tex": "liquid_purple",
        "tex": "wall_lab",
        "heights": (8.0, 16.0),
        "landmark": {"tex": "neon_good_boy", "pos": [36.0, 1.2, 18.0], "px": 0.024},
        "signs": ["neon_good_boy", "neon_boys", "neon_catface", "neon_love_wins"],
    },
    {
        "id": "silent_archive",
        "name_ru": "Тихий архив",
        "name_en": "Silent Archive",
        "center": [-40.0, -38.0],
        "radius": 24.0,
        "color": [0.62, 0.36, 1.0],
        "outline_tex": "liquid_purple",
        "tex": "wall_archive",
        "heights": (13.0, 23.0),
        "landmark": {"tex": "neon_trans_rights", "pos": [-40.0, 1.2, -32.0], "px": 0.022},
        "signs": ["neon_trans_rights", "neon_hearts", "neon_uwu"],
    },
    {
        "id": "foundry_ring",
        "name_ru": "Литейное кольцо",
        "name_en": "Foundry Ring",
        "center": [40.0, -38.0],
        "radius": 24.0,
        "color": [1.0, 0.62, 0.16],
        "outline_tex": "liquid_red",
        "tex": "wall_boss",
        "heights": (11.0, 20.0),
        "landmark": {"tex": "neon_game_over", "pos": [40.0, 1.2, -32.0], "px": 0.022},
        "signs": ["neon_game_over", "neon_traps", "neon_boys"],
    },
    {
        "id": "descent_shrine",
        "name_ru": "Святилище Спуска",
        "name_en": "Descent Shrine",
        "center": [0.0, -56.0],
        "radius": 18.0,
        "color": [0.85, 0.2, 0.45],
        "outline_tex": "liquid_black",
        "tex": "wall_main",
        "heights": (9.0, 15.0),
        "landmark": {"tex": "neon_game_over", "pos": [0.0, 1.4, -46.0], "px": 0.02},
        "signs": ["neon_game_over", "neon_heart"],
    },
]

# Улицы — прямоугольники (x0, z0, x1, z1). По ним не строят, вдоль них ставят
# фасады, фонари и неоновые полосы.
STREETS = [
    ("boulevard", -7.0, -78.0, 7.0, 78.0),          # главный проспект север-юг
    ("crossline", -78.0, -7.0, 78.0, 7.0),          # поперечная ось запад-восток
    ("north_ave", -66.0, 10.5, 66.0, 17.5),         # рынок ↔ милосердие
    ("south_ave", -66.0, -41.5, 66.0, -34.5),       # архив ↔ литейка
    ("ring_w", -59.0, -59.0, -53.0, 59.0),
    ("ring_e", 53.0, -59.0, 59.0, 59.0),
    ("ring_s", -59.0, -59.0, 59.0, -53.0),
    ("ring_n", -59.0, 53.0, 59.0, 59.0),
    # Внешнее кольцо: без него пояс башен превращается в сплошную стену,
    # по которой не пройти.
    ("outer_w", -75.0, -75.0, -69.0, 75.0),
    ("outer_e", 69.0, -75.0, 75.0, 75.0),
    ("outer_s", -75.0, -75.0, 75.0, -69.0),
    ("outer_n", -75.0, 69.0, 75.0, 75.0),
]

# Маяки на крышах: id, точка, высота, цвет, текстура свечения, венец, размер.
BEACONS = [
    ("heart_tower", (0.0, -3.0), 18.0, [1.0, 0.35, 0.7], "liquid_pink", "neon_heart", 0.02),
    ("market_kawaii", (-48.0, 22.0), 12.0, [1.0, 0.5, 0.2], "liquid_red", "neon_kawaii", 0.018),
    ("market_pills", (-24.0, 33.0), 9.0, [1.0, 0.6, 0.3], "liquid_red", "neon_pills", 0.016),
    ("mercy_signal", (48.0, 22.0), 13.0, [0.3, 0.85, 1.0], "liquid_purple", "neon_good_boy", 0.018),
    ("mercy_heart", (24.0, 34.0), 10.0, [0.4, 0.8, 1.0], "liquid_purple", "neon_hearts", 0.016),
    ("archive_spire", (-52.0, -44.0), 15.0, [0.62, 0.36, 1.0], "liquid_purple",
     "neon_trans_rights", 0.018),
    ("archive_memory", (-28.0, -54.0), 11.0, [0.55, 0.4, 1.0], "liquid_purple", "neon_uwu", 0.016),
    ("foundry_stack", (52.0, -44.0), 16.0, [1.0, 0.6, 0.15], "liquid_red", "neon_game_over", 0.018),
    ("foundry_warning", (28.0, -54.0), 12.0, [1.0, 0.45, 0.15], "liquid_red", "neon_traps", 0.016),
    ("descent_axis", (0.0, -70.0), 20.0, [0.9, 0.18, 0.4], "liquid_black", "neon_game_over", 0.02),
    ("north_gate", (0.0, 66.0), 17.0, [1.0, 0.4, 0.75], "liquid_pink", "neon_love_wins", 0.018),
    ("west_gate", (-66.0, 0.0), 19.0, [0.7, 0.4, 1.0], "liquid_purple", "neon_femboy", 0.018),
    ("east_gate", (66.0, 0.0), 19.0, [0.35, 0.8, 1.0], "liquid_purple", "neon_boys", 0.018),
    ("market_corner", (-58.0, 52.0), 14.0, [1.0, 0.5, 0.25], "liquid_red", "neon_catface", 0.016),
    ("mercy_corner", (58.0, 52.0), 14.0, [0.35, 0.85, 1.0], "liquid_purple", "neon_uwu", 0.016),
    ("archive_corner", (-58.0, -52.0), 15.0, [0.6, 0.35, 1.0], "liquid_purple", "neon_hearts",
     0.016),
]

# Куда строить нельзя ни при каких условиях: (x, z, радиус).
def keepouts() -> list[tuple[float, float, float]]:
    zones: list[tuple[float, float, float]] = [
        (0.0, 0.0, 20.0),        # площадь целиком открыта
        (0.0, -56.0, 19.0),      # терраса святилища и врата
        (PLAYER_SPAWN[0], PLAYER_SPAWN[2], 7.0),
    ]
    for d in DISTRICTS:
        # Внутри каждого района — своя открытая площадка.
        zones.append((d["center"][0], d["center"][1], 11.0))
        mark = d["landmark"]
        zones.append((mark["pos"][0], mark["pos"][2], 4.0))
    for npc in json.load(io.open(NPCS_PATH, encoding="utf-8")):
        pos = npc.get("pos")
        if pos:
            zones.append((float(pos[0]), float(pos[1]), 5.0))
    for beacon in BEACONS:
        zones.append((beacon[1][0], beacon[1][1], 5.0))
    return zones


# ── Мелкие помощники ─────────────────────────────────────────────────────────


def r1(value: float) -> float:
    return round(float(value) + 0.0, 2)


def v3(x: float, y: float, z: float) -> list[float]:
    return [r1(x), r1(y), r1(z)]


def in_rect(x: float, z: float, rect, pad: float = 0.0) -> bool:
    _, x0, z0, x1, z1 = rect
    return x0 - pad <= x <= x1 + pad and z0 - pad <= z <= z1 + pad


def street_at(x: float, z: float, pad: float = 0.0):
    for rect in STREETS:
        if in_rect(x, z, rect, pad):
            return rect
    return None


class Map:
    """Собираемая карта плюс учёт того, что нельзя посчитать задним числом."""

    def __init__(self, rng: random.Random):
        self.rng = rng
        self.blocks: list[dict] = []
        self.buildings: list[dict] = []
        self.props: list[dict] = []
        self.flats: list[dict] = []
        self.lights: list[dict] = []
        self.glows: list[dict] = []
        self.routes: list[dict] = []
        self.beacons: list[dict] = []
        self.clusters: list[dict] = []
        self.occupied: list[tuple[float, float, float]] = []  # x, z, радиус
        self.tile_lights: dict[tuple[int, int], int] = defaultdict(int)
        self.skipped_lights = 0

    # Свет считаем по плиткам земли: мобильный рендер не тянет много ламп на
    # один объект, а земля разбита ровно на плитки 25×25.
    LIGHT_TILE = 25.0
    LIGHT_PER_TILE = 6

    def light(self, pos, color, energy: float, rng: float) -> bool:
        key = (int(math.floor(pos[0] / self.LIGHT_TILE)), int(math.floor(pos[2] / self.LIGHT_TILE)))
        if self.tile_lights[key] >= self.LIGHT_PER_TILE:
            self.skipped_lights += 1
            return False
        self.tile_lights[key] += 1
        self.lights.append(
            {
                "pos": [r1(pos[0]), r1(pos[1]), r1(pos[2])],
                "color": [r1(c) for c in color],
                "energy": r1(energy),
                "range": r1(rng),
            }
        )
        return True

    def reserve_light(self, x: float, z: float) -> bool:
        """Занять слот под лампу, которую поставит не карта, а билдер.

        Вывеска здания зажигает свою лампу уже в движке
        (`client/src/worldgen/map.rs`), и если её не считать, бюджет на плитку
        врёт ровно там, где неона больше всего.
        """
        key = (int(math.floor(x / self.LIGHT_TILE)), int(math.floor(z / self.LIGHT_TILE)))
        if self.tile_lights[key] >= self.LIGHT_PER_TILE:
            return False
        self.tile_lights[key] += 1
        return True

    def free(self, x: float, z: float, radius: float) -> bool:
        for ox, oz, orad in self.occupied:
            if (x - ox) ** 2 + (z - oz) ** 2 < (radius + orad) ** 2:
                return False
        return True

    def take(self, x: float, z: float, radius: float) -> None:
        self.occupied.append((x, z, radius))


# ── Земля, площадь, святилище ────────────────────────────────────────────────


def build_plaza(m: Map) -> None:
    """Площадь Сердца: приподнятый круг, ступени с четырёх сторон, фонтан."""
    m.blocks.append(
        {"shape": "cylinder", "pos": v3(0, 0, 0), "radius": 14.0, "height": 0.55,
         "tex": "floor_main", "uv": 5.0}
    )
    # Постамента у фонтана нет: через центр площади идёт дорога от спавна к
    # вратам, и любой бортик её перекрывает.
    # Подходы: с проспектов и поперечной оси на круг ведут пологие пандусы.
    # Ступени тут не годятся — персонаж на них просто упирается.
    for dx, dz in ((0, 1), (0, -1), (1, 0), (-1, 0)):
        m.blocks.append(
            {
                "shape": "ramp",
                "from": v3(dx * 19.5, 0.0, dz * 19.5),
                "to": v3(dx * 13.0, 0.55, dz * 13.0),
                "width": 9.0,
                "tex": "floor_main",
                "uv": 3.0,
            }
        )
    # Неоновое кольцо по краю круга плюс восемь фонарей.
    for i in range(24):
        a = i / 24.0 * math.tau
        m.glows.append(
            {
                "pos": v3(math.cos(a) * 13.7, 0.6, math.sin(a) * 13.7),
                "size": [1.9, 0.06, 1.9],
                "tex": "liquid_pink",
                "emission": [1.0, 0.34, 0.62],
                "uv": 1.0,
            }
        )
    for i in range(8):
        a = (i + 0.5) / 8.0 * math.tau
        x, z = math.cos(a) * 15.8, math.sin(a) * 15.8
        m.blocks.append(
            {"shape": "cylinder", "pos": v3(x, 0, z), "radius": 0.22, "height": 5.2,
             "tex": "wall_main", "uv": 2.0}
        )
        m.glows.append(
            {"pos": v3(x, 5.35, z), "size": [0.5, 0.3, 0.5], "tex": "liquid_pink",
             "emission": [1.0, 0.45, 0.72], "uv": 1.0}
        )
        m.take(x, z, 1.2)
    m.props.append({"tex": "street_fountain", "pos": v3(0, 0.55, 0), "px": 0.03})
    m.take(0, 0, 5.5)
    m.light([0.0, 4.2, 0.0], [1.0, 0.5, 0.78], 1.7, 20.0)


def build_shrine(m: Map) -> None:
    """Святилище Спуска: терраса с вратами, к ней ведут пандусы и лестница."""
    m.blocks.append(
        {"shape": "box", "pos": v3(0, 1.3, -58.0), "size": [30.0, 2.6, 18.0],
         "rot": 0.0, "tex": "dtile_02", "uv": 3.0}
    )
    # Главный подъём — широкий пандус по оси проспекта: по нему игрок и
    # приходит к вратам.
    m.blocks.append(
        {"shape": "ramp", "from": v3(0, 0.0, -39.0), "to": v3(0, 2.6, -49.0),
         "width": 11.0, "tex": "dtile_02", "uv": 4.0}
    )
    for side in (-1, 1):
        # Боковые пандусы должны заканчиваться НА террасе (её край — x = ±15),
        # иначе с них некуда шагнуть.
        m.blocks.append(
            {"shape": "ramp", "from": v3(side * 11.0, 0.0, -41.5),
             "to": v3(side * 11.0, 2.6, -49.0), "width": 4.0, "tex": "dtile_02", "uv": 3.0}
        )
        # Пилоны врат: две колонны, между ними проход к порталу.
        m.blocks.append(
            {"shape": "box", "pos": v3(side * 6.4, 5.2, -60.5), "size": [2.2, 10.4, 2.2],
             "rot": 0.0, "tex": "wall_boss", "uv": 2.0}
        )
        m.glows.append(
            {"pos": v3(side * 6.4, 10.6, -60.5), "size": [2.6, 0.4, 2.6], "tex": "liquid_red",
             "emission": [0.95, 0.2, 0.3], "uv": 1.0}
        )
        m.light([side * 6.4, 9.0, -59.0], [1.0, 0.28, 0.36], 1.5, 13.0)
        m.take(side * 6.4, -60.5, 2.5)
    # Кромка террасы, а не залитая светом плита: иначе площадка выглядит
    # куском цветной бумаги.
    for pos, size in (
        ((0.0, -49.4), (28.0, 0.4)),
        ((0.0, -66.6), (28.0, 0.4)),
        ((-14.8, -58.0), (0.4, 17.6)),
        ((14.8, -58.0), (0.4, 17.6)),
    ):
        m.glows.append(
            {"pos": v3(pos[0], 2.68, pos[1]), "size": [size[0], 0.06, size[1]],
             "tex": "liquid_red", "emission": [0.65, 0.14, 0.3], "uv": 4.0}
        )
    m.take(0, -58.0, 12.0)


def build_skywalk(m: Map) -> None:
    """Эстакада над поперечной осью: сверху виден весь квартал."""
    y = 7.4
    for x0, x1 in ((-52.0, -20.0), (20.0, 52.0)):
        m.blocks.append(
            {"shape": "box", "pos": v3((x0 + x1) / 2, y, 0.0), "size": [x1 - x0, 0.4, 5.0],
             "rot": 0.0, "tex": "dtile_07", "uv": 4.0}
        )
        for side in (-1, 1):
            m.glows.append(
                {
                    "pos": v3((x0 + x1) / 2, y + 0.5, side * 2.4),
                    "size": [x1 - x0, 0.07, 0.08],
                    "tex": "liquid_purple",
                    "emission": [0.22, 0.4, 0.6],
                    "uv": 8.0,
                }
            )
        # Опоры — портальными рамами по краям проспекта: столб посреди
        # проезжей части останавливает игрока на ровном месте.
        for t in (0.15, 0.5, 0.85):
            x = x0 + (x1 - x0) * t
            for side in (-1, 1):
                m.blocks.append(
                    {"shape": "cylinder", "pos": v3(x, 0, side * 5.6), "radius": 0.5,
                     "height": y, "tex": "wall_main", "uv": 3.0}
                )
                m.take(x, side * 5.6, 1.4)
            # Ригель, на котором лежит настил.
            m.blocks.append(
                {"shape": "box", "pos": v3(x, y - 0.45, 0.0), "size": [1.0, 0.5, 12.2],
                 "rot": 0.0, "tex": "wall_main", "uv": 3.0}
            )
    # Съезды на площадь и на кольцо.
    for side in (-1, 1):
        m.blocks.append(
            {"shape": "ramp", "from": v3(side * 20.0, y, 0.0), "to": v3(side * 15.5, 0.55, 0.0),
             "width": 4.6, "tex": "dtile_07", "uv": 4.0}
        )
        m.blocks.append(
            {"shape": "stairs", "from": v3(side * 52.0, y, 0.0), "to": v3(side * 56.0, 0.0, 0.0),
             "width": 4.6, "steps": 8, "tex": "dtile_07", "uv": 1.0}
        )
    m.light([-36.0, y + 2.0, 0.0], [0.45, 0.75, 1.0], 1.1, 14.0)
    m.light([36.0, y + 2.0, 0.0], [0.45, 0.75, 1.0], 1.1, 14.0)


# ── Кварталы ─────────────────────────────────────────────────────────────────


def district_of(x: float, z: float):
    """Ближайший район, если участок в его зоне влияния."""
    best, best_d = None, 1e9
    for d in DISTRICTS:
        dx, dz = x - d["center"][0], z - d["center"][1]
        dist = math.hypot(dx, dz)
        if dist < d["radius"] + 14.0 and dist < best_d:
            best, best_d = d, dist
    return best


def build_lots(m: Map, zones) -> None:
    """Нарезать свободную землю на участки и застроить их.

    Сетка, а не случайные точки: город должен читаться улицами, а дома —
    стоять фасадом к проезжей части, иначе получается свалка коробок.
    """
    n = int(CITY * 2 // CELL)
    origin = -n * CELL / 2.0

    def cell_free(ix: int, iz: int) -> bool:
        cx = origin + (ix + 0.5) * CELL
        cz = origin + (iz + 0.5) * CELL
        if max(abs(cx), abs(cz)) > CITY:
            return False
        if street_at(cx, cz, CELL * 0.5 - GAP * 0.5):
            return False
        for ox, oz, orad in zones:
            if math.hypot(cx - ox, cz - oz) < orad + CELL * 0.45:
                return False
        return True

    used = [[False] * n for _ in range(n)]
    lots = []
    for ix in range(n):
        for iz in range(n):
            if used[ix][iz] or not cell_free(ix, iz):
                continue
            # Пробуем вырастить участок 2×2 → 2×1 → 1×2 → 1×1.
            for w, d in m.rng.choice([[(2, 2), (2, 1), (1, 2), (1, 1)],
                                      [(2, 1), (2, 2), (1, 1), (1, 2)],
                                      [(1, 2), (2, 2), (1, 1), (2, 1)]]):
                if ix + w > n or iz + d > n:
                    continue
                if all(
                    not used[ix + a][iz + b] and cell_free(ix + a, iz + b)
                    for a in range(w)
                    for b in range(d)
                ):
                    for a in range(w):
                        for b in range(d):
                            used[ix + a][iz + b] = True
                    cx = origin + (ix + w / 2.0) * CELL
                    cz = origin + (iz + d / 2.0) * CELL
                    lots.append((cx, cz, w * CELL - GAP, d * CELL - GAP))
                    break

    for cx, cz, w, d in lots:
        place_building(m, cx, cz, w, d)


def facing_street(cx: float, cz: float, w: float, d: float):
    """С какой стороны участок выходит на улицу: n|s|e|w и точка на фасаде."""
    checks = [
        ("s", cx, cz + d / 2.0 + 3.0),
        ("n", cx, cz - d / 2.0 - 3.0),
        ("e", cx + w / 2.0 + 3.0, cz),
        ("w", cx - w / 2.0 - 3.0, cz),
    ]
    for side, px, pz in checks:
        if street_at(px, pz):
            return side
    return None


SIDE_NORMAL = {"s": (0.0, 1.0, 0.0), "n": (0.0, -1.0, 180.0), "e": (1.0, 0.0, 90.0),
               "w": (-1.0, 0.0, -90.0)}

OUTER_TEX = ["wall_archive", "wall_lab", "wall_main", "wall_boss", "wall_market"]
OUTER_SIGNS = ["neon_hearts", "neon_boys", "neon_catface", "neon_love_wins", "neon_traps",
               "neon_kawaii", "neon_femboy", "neon_uwu"]


def place_building(m: Map, cx: float, cz: float, w: float, d: float) -> None:
    district = district_of(cx, cz)
    edge = max(abs(cx), abs(cz)) > 60.0
    if edge:
        # Пояс башен по краю: он и есть горизонт города, без него карта
        # выглядит коробками на пустой плите.
        low, high = 22.0, 38.0
        tex = OUTER_TEX[int(abs(cx * 7 + cz * 3)) % len(OUTER_TEX)]
        # Башни пояса тоже с неоном, но реже: это фон, а не витрина.
        signs = OUTER_SIGNS
    elif district:
        low, high = district["heights"]
        tex = district["tex"]
        signs = district["signs"]
    else:
        low, high = 8.0, 15.0
        tex = "wall_main"
        signs = ["neon_hearts", "neon_uwu"]

    height = round(m.rng.uniform(low, high), 1)
    m.buildings.append({"pos": [r1(cx), r1(cz)], "size": [r1(w), height, r1(d)], "tex": tex})
    m.take(cx, cz, max(w, d) * 0.5)
    entry = m.buildings[-1]

    # Второй ярус с отступом: силуэт перестаёт быть частоколом одинаковых плит.
    if height > 12.0 and m.rng.random() < 0.65:
        th = round(m.rng.uniform(2.5, height * 0.45), 1)
        m.blocks.append(
            {
                "shape": "box",
                "pos": v3(cx, height + th / 2.0, cz),
                "size": [r1(w * 0.62), th, r1(d * 0.62)],
                "rot": 0.0,
                "tex": tex,
                "uv": 3.0,
            }
        )
        height += th

    side = facing_street(cx, cz, w, d)
    if side is None:
        return

    nx, nz, rot = SIDE_NORMAL[side]
    # Светящийся карниз по верху фасада и полоса по низу — вся «неоновость»
    # держится на них, они бесплатные (unshaded), в отличие от ламп.
    color = district["color"] if district else [0.6, 0.45, 1.0]
    m.glows.append(
        {
            "pos": v3(cx + nx * (w / 2.0 + 0.05), height - 0.5, cz + nz * (d / 2.0 + 0.05)),
            "size": [r1(w * 0.9) if nx == 0 else 0.12, 0.22, 0.12 if nx == 0 else r1(d * 0.9)],
            "tex": "liquid_pink" if nx == 0 else "liquid_purple",
            "emission": [r1(color[0]), r1(color[1]), r1(color[2])],
            "uv": 4.0,
        }
    )
    # Отблеск на тротуаре: узкая полоса, а не ковёр под всю витрину.
    m.glows.append(
        {
            "pos": v3(cx + nx * (w / 2.0 + 0.45), 0.08, cz + nz * (d / 2.0 + 0.45)),
            "size": [r1(w * 0.8) if nx == 0 else 0.5, 0.05, 0.5 if nx == 0 else r1(d * 0.8)],
            "tex": "liquid_purple",
            "emission": [r1(color[0] * 0.32), r1(color[1] * 0.32), r1(color[2] * 0.38)],
            "uv": 3.0,
        }
    )

    # Ряды светящихся окон: главное, что отличает дом от чёрной коробки.
    rows = int(min(4, max(1, height // 4.5)))
    # Разброс по этажам и по яркости: одинаковые полосы на всех домах читаются
    # как обои, а не как окна.
    jitter = m.rng.uniform(-0.6, 0.9)
    warmth = m.rng.uniform(0.7, 1.25)
    for row in range(rows):
        y = 2.6 + jitter + row * (height - 3.4) / max(1, rows)
        if y > height - 1.0 or y < 1.6:
            continue
        m.glows.append(
            {
                "pos": v3(cx + nx * (w / 2.0 + 0.06), y, cz + nz * (d / 2.0 + 0.06)),
                "size": [r1(w * 0.78) if nx == 0 else 0.09, 0.8,
                         0.09 if nx == 0 else r1(d * 0.78)],
                "tex": "dtile_11",
                "emission": [r1((color[0] * 0.4 + 0.3) * warmth),
                             r1((color[1] * 0.4 + 0.26) * warmth),
                             r1((color[2] * 0.4 + 0.36) * warmth)],
                "uv": 6.0,
            }
        )

    # Вывеска — только на части домов: если светится каждый, не светится ни один.
    # И только пока на плитке есть место под лампу: движок зажигает её сам.
    chance = 0.16 if edge else 0.34
    wants_sign = bool(signs) and m.rng.random() < chance
    if wants_sign and m.reserve_light(cx, cz):
        entry["sign"] = signs[m.rng.randrange(len(signs))]
        entry["sign_side"] = side
    elif signs and m.rng.random() < 0.55:
        # Плоский неон над входом — дешевле вывески: без лампы.
        m.flats.append(
            {
                "tex": signs[m.rng.randrange(len(signs))],
                "pos": v3(cx + nx * (w / 2.0 + 0.12), 2.4, cz + nz * (d / 2.0 + 0.12)),
                "rot": r1(rot),
                "px": 0.018,
                "glow": True,
            }
        )

    # Тротуарная мелочёвка вдоль фасада.
    street_props = ["street_bench", "street_trashcan", "street_vending", "street_phone",
                    "street_bags", "street_cone", "street_dumpster", "street_fan_unit",
                    "street_grate_table"]
    for _ in range(m.rng.randint(0, 2)):
        along = m.rng.uniform(-0.34, 0.34)
        px = cx + nx * (w / 2.0 + 1.5) + (0.0 if nx else w * along)
        pz = cz + nz * (d / 2.0 + 1.5) + (0.0 if nz else d * along)
        if not m.free(px, pz, 0.8):
            continue
        m.props.append(
            {"tex": street_props[m.rng.randrange(len(street_props))], "pos": v3(px, 0.0, pz),
             "px": 0.022}
        )
        m.take(px, pz, 0.8)


# ── Улицы: фонари, полосы, маршруты ──────────────────────────────────────────


def build_streets(m: Map) -> None:
    """Фонари вдоль проезжей части и светящаяся разметка по осям."""
    lamp_color = [1.0, 0.55, 0.8]
    for name, x0, z0, x1, z1 in STREETS:
        horizontal = (x1 - x0) > (z1 - z0)
        length = (x1 - x0) if horizontal else (z1 - z0)
        step = 16.0
        count = max(2, int(length // step))
        for i in range(count + 1):
            t = i / count
            if horizontal:
                x = x0 + (x1 - x0) * t
                base = (z0 + z1) / 2.0
                offsets = ((base - (z1 - z0) / 2.0 - 1.2), (base + (z1 - z0) / 2.0 + 1.2))
                spots = [(x, o) for o in offsets]
            else:
                z = z0 + (z1 - z0) * t
                base = (x0 + x1) / 2.0
                offsets = ((base - (x1 - x0) / 2.0 - 1.2), (base + (x1 - x0) / 2.0 + 1.2))
                spots = [(o, z) for o in offsets]
            for px, pz in spots:
                if max(abs(px), abs(pz)) > CITY + 6.0 or not m.free(px, pz, 1.0):
                    continue
                if math.hypot(px, pz) < 15.0:
                    continue  # площадь освещена своими фонарями
                # Фонарь стоит на краю своей улицы. Если этот край попал внутрь
                # другой улицы — это перекрёсток, и столб встанет посреди
                # проезжей части: игрок упрётся в него на ровном месте.
                if street_at(px, pz, 1.0):
                    continue
                m.blocks.append(
                    {"shape": "cylinder", "pos": v3(px, 0, pz), "radius": 0.18, "height": 4.4,
                     "tex": "wall_main", "uv": 2.0}
                )
                m.glows.append(
                    {"pos": v3(px, 4.5, pz), "size": [0.42, 0.26, 0.42], "tex": "liquid_pink",
                     "emission": [1.0, 0.5, 0.78], "uv": 1.0}
                )
                m.take(px, pz, 1.0)
                if (i % 2) == 0:
                    m.light([px, 4.3, pz], lamp_color, 0.9, 9.0)

        # Светящаяся осевая линия.
        if horizontal:
            m.glows.append(
                {"pos": v3((x0 + x1) / 2.0, 0.03, (z0 + z1) / 2.0),
                 "size": [r1(x1 - x0), 0.05, 0.4], "tex": "liquid_pink",
                 "emission": [0.4, 0.13, 0.26], "uv": r1((x1 - x0) / 6.0)}
            )
        else:
            m.glows.append(
                {"pos": v3((x0 + x1) / 2.0, 0.03, (z0 + z1) / 2.0),
                 "size": [0.4, 0.05, r1(z1 - z0)], "tex": "liquid_pink",
                 "emission": [0.4, 0.13, 0.26], "uv": r1((z1 - z0) / 6.0)}
            )


def build_routes(m: Map) -> None:
    """Маршрутные линии: подсказка, куда идти, поверх разметки."""
    def route(rid, pts, color, tex, width=1.5):
        m.routes.append(
            {
                "id": rid,
                "points": [v3(*p) for p in pts],
                "width": width,
                "color": [r1(c * 0.42) for c in color],
                "glow_tex": tex,
                "uv": 1.2,
            }
        )

    route("heart_boulevard",
          [(0, 0.0, 12), (0, 0.6, 2), (0, 0.0, -20), (0, 0.0, -44), (0, 2.68, -55)],
          [1.0, 0.3, 0.72], "liquid_pink", 1.7)
    route("market_spur", [(0, 0.6, 2), (-16, 0.0, 8), (-28, 0.0, 14), (-34, 0.0, 14)],
          [1.0, 0.48, 0.2], "liquid_red")
    route("mercy_spur", [(0, 0.6, 2), (16, 0.0, 8), (30, 0.0, 14), (36, 0.0, 14)],
          [0.25, 0.85, 1.0], "liquid_purple")
    route("archive_spur", [(0, 0.0, -20), (-18, 0.0, -28), (-32, 0.0, -38), (-40, 0.0, -38)],
          [0.62, 0.36, 1.0], "liquid_purple")
    route("foundry_spur", [(0, 0.0, -20), (18, 0.0, -28), (32, 0.0, -38), (40, 0.0, -38)],
          [1.0, 0.62, 0.16], "liquid_red")
    route("west_loop", [(-56, 0.0, 40), (-56, 0.0, 0), (-56, 0.0, -40)],
          [0.8, 0.4, 1.0], "liquid_purple", 1.2)
    route("east_loop", [(56, 0.0, 40), (56, 0.0, 0), (56, 0.0, -40)],
          [0.4, 0.8, 1.0], "liquid_purple", 1.2)
    route("transit_crossline", [(-52, 7.9, 0), (-20, 7.9, 0), (20, 7.9, 0), (52, 7.9, 0)],
          [0.45, 0.75, 1.0], "liquid_purple", 1.1)
    route("north_promenade", [(-40, 0.0, 14), (-12, 0.0, 14), (12, 0.0, 14), (40, 0.0, 14)],
          [1.0, 0.4, 0.6], "liquid_pink", 1.0)
    route("south_works", [(-40, 0.0, -38), (-12, 0.0, -38), (12, 0.0, -38), (40, 0.0, -38)],
          [1.0, 0.5, 0.3], "liquid_red", 1.0)


def build_beacons(m: Map) -> None:
    """Маяки на крышах: по ним ориентируешься с любой точки квартала."""
    for bid, pos, height, color, tex, crown, px in BEACONS:
        m.beacons.append(
            {
                "id": bid,
                "pos": [r1(pos[0]), r1(pos[1])],
                "height": r1(height),
                "color": [r1(c) for c in color],
                "glow_tex": tex,
                "crown": crown,
                "crown_px": px,
            }
        )
        m.take(pos[0], pos[1], 2.5)
        # Маяк зажигает лампу на крыше уже в движке — место под неё занимаем здесь.
        m.reserve_light(pos[0], pos[1])


def build_clusters(m: Map) -> None:
    """Живая мелочь: то, что покачивается и крутится в центре районов."""
    spec = [
        ("heart_plaza_social", (0.0, 0.0), 11.0, 12, [1.0, 0.32, 0.72], "liquid_pink",
         ["street_bench", "street_fountain", "neon_hearts", "street_phone"], 0.18, 5.0, 0.8),
        ("night_market_stalls", (-34.0, 14.0), 17.0, 16, [1.0, 0.46, 0.18], "liquid_red",
         ["street_vending", "street_bags", "street_grate_table", "neon_kawaii", "neon_pills"],
         0.34, -9.0, 1.35),
        ("mercy_ward_rest", (36.0, 14.0), 16.0, 13, [0.28, 0.82, 1.0], "liquid_purple",
         ["street_bench", "bath_sink", "furn_chair", "neon_good_boy"], 0.22, 4.0, 0.9),
        ("silent_archive_relics", (-40.0, -38.0), 17.0, 12, [0.62, 0.36, 1.0], "liquid_purple",
         ["furn_desk", "furn_dresser", "neon_trans_rights", "street_phone"], 0.16, 3.0, 0.7),
        ("foundry_service_ring", (40.0, -38.0), 17.0, 14, [1.0, 0.62, 0.16], "liquid_red",
         ["street_dumpster", "street_cone", "street_fan_unit", "neon_game_over"], 0.26, -6.0, 1.1),
        ("descent_shrine_votives", (0.0, -56.0), 13.0, 10, [0.85, 0.2, 0.45], "liquid_black",
         ["soul", "neon_heart", "street_bench"], 0.42, 7.0, 0.6),
    ]
    for cid, center, radius, density, color, tex, props, bob, spin, speed in spec:
        m.clusters.append(
            {
                "id": cid,
                "center": [r1(center[0]), r1(center[1])],
                "radius": r1(radius),
                "density": density,
                "color": [r1(c) for c in color],
                "glow_tex": tex,
                "props": props,
                "bob": r1(bob),
                "spin": r1(spin),
                "speed": r1(speed),
            }
        )


def build_ambient(m: Map) -> None:
    """Лужи неона и подсветка перекрёстков — то, что делает мокрый асфальт."""
    crossings = [(0.0, 14.0), (0.0, -38.0), (-34.0, 0.0), (36.0, 0.0),
                 (-56.0, 14.0), (56.0, 14.0), (-56.0, -38.0), (56.0, -38.0),
                 (0.0, 56.0), (-56.0, 0.0), (56.0, 0.0)]
    for x, z in crossings:
        m.glows.append(
            {"pos": v3(x, 0.04, z), "size": [7.0, 0.05, 7.0], "tex": "liquid_purple",
             "emission": [0.22, 0.17, 0.44], "uv": 2.0}
        )
        m.light([x, 3.4, z], [0.75, 0.5, 1.0], 1.0, 12.0)

    rng = m.rng
    for _ in range(46):
        x = rng.uniform(-CITY, CITY)
        z = rng.uniform(-CITY, CITY)
        if not street_at(x, z):
            continue
        size = round(rng.uniform(1.6, 4.4), 1)
        m.glows.append(
            {
                "pos": v3(x, 0.02, z),
                "size": [size, 0.04, round(size * rng.uniform(0.6, 1.3), 1)],
                "tex": rng.choice(["liquid_pink", "liquid_purple", "liquid_red"]),
                "emission": [round(rng.uniform(0.16, 0.34), 2), round(rng.uniform(0.1, 0.22), 2),
                             round(rng.uniform(0.24, 0.44), 2)],
                "uv": 2.0,
            }
        )

    # Души-огоньки вдоль дороги к вратам: тропа к спуску должна читаться.
    for i in range(9):
        z = -22.0 - i * 4.0
        for side in (-1, 1):
            m.props.append({"tex": "soul", "pos": v3(side * 5.8, 0.9, z), "px": 0.011})


# ── Сборка ───────────────────────────────────────────────────────────────────


def build(spawns: dict) -> dict:
    rng = random.Random(SEED)
    m = Map(rng)
    zones = keepouts()
    for x, z, kind in [(s["x"], s["z"], "enemy") for s in spawns.get("spawn_enemies", [])]:
        zones.append((float(x), float(z), 6.0))

    build_plaza(m)
    build_shrine(m)
    build_beacons(m)
    build_lots(m, zones)
    build_streets(m)
    build_skywalk(m)
    build_ambient(m)
    build_routes(m)
    build_clusters(m)

    districts = []
    for d in DISTRICTS:
        districts.append(
            {
                "id": d["id"],
                "name_ru": d["name_ru"],
                "name_en": d["name_en"],
                "center": [r1(d["center"][0]), r1(d["center"][1])],
                "radius": r1(d["radius"]),
                "color": [r1(c) for c in d["color"]],
                "outline_tex": d["outline_tex"],
                "landmark": {
                    "tex": d["landmark"]["tex"],
                    "pos": [r1(v) for v in d["landmark"]["pos"]],
                    "px": d["landmark"]["px"],
                },
            }
        )

    return {
        "id": "hub",
        "name_ru": "Неоновый квартал",
        "name_en": "Neon Quarter",
        "env": {
            "sky": "sky_purple",
            # Туман гуще 0.006 съедает силуэт города уже с середины проспекта.
            "fog_density": 0.005,
            # Ночь, но не чернота: при меньшем ambient дома превращаются в
            # силуэты, и от квартала остаются одни неоновые полоски.
            "ambient": [0.55, 0.45, 0.62],
            "ambient_energy": 1.5,
        },
        "player_spawn": PLAYER_SPAWN,
        "gate": GATE,
        "ground": {
            "size": GROUND,
            "tex": "floor_main",
            "uv": 4.0,
            "border_h": 18.0,
            "border_tex": "wall_arena",
        },
        "districts": districts,
        "decor_clusters": m.clusters,
        "route_layers": m.routes,
        "skyline_beacons": m.beacons,
        "blocks": m.blocks,
        "buildings": m.buildings,
        "glows": m.glows,
        "props": m.props,
        "flats": m.flats,
        "lights": m.lights,
        "spawns": spawns,
    }


def validate(m: dict, spawns: dict) -> list[str]:
    """Проверки, которые дешевле сделать здесь, чем заметить в игре."""
    problems = []

    boxes = []
    for b in m["buildings"]:
        boxes.append((b["pos"][0], b["pos"][1], b["size"][0], b["size"][2]))

    def blocked(x: float, z: float, clearance: float) -> bool:
        for bx, bz, w, d in boxes:
            if abs(x - bx) < w / 2.0 + clearance and abs(z - bz) < d / 2.0 + clearance:
                return True
        return False

    for npc in json.load(io.open(NPCS_PATH, encoding="utf-8")):
        pos = npc.get("pos")
        if pos and blocked(float(pos[0]), float(pos[1]), 1.2):
            problems.append(f"NPC {npc['id']} стоит в стене")
    for s in spawns.get("spawn_enemies", []):
        if blocked(float(s["x"]), float(s["z"]), 1.2):
            problems.append(f"спавн {s['kind']} ({s['x']}, {s['z']}) внутри дома")
    for point in (m["player_spawn"][0], 0.0), (0.0, m["gate"][2]):
        if blocked(point[0], point[1], 1.5):
            problems.append(f"точка входа {point} перекрыта")

    # Проспект и поперечная ось — единственная дорога от спавна к вратам:
    # ни один столб не должен стоять в полосе движения.
    lanes = (("проспект", 4.0, None), ("поперечная ось", None, 4.0))
    for name, half_x, half_z in lanes:
        for block in m["blocks"]:
            if block["shape"] == "cylinder":
                x, _, z = block["pos"]
                footprint = block["radius"]
            elif block["shape"] == "box":
                x, y, z = block["pos"]
                footprint = min(block["size"][0], block["size"][2]) / 2.0
                # Настил эстакады и терраса святилища — над головой или под
                # ногами, они дорогу не перекрывают.
                if y - block["size"][1] / 2.0 > 2.5 or block["size"][1] <= 0.6:
                    continue
            else:
                continue
            if footprint > 2.0:
                continue  # это не столб, а массив: терраса, постамент
            in_lane = (half_x is not None and abs(x) < half_x + footprint) or (
                half_z is not None and abs(z) < half_z + footprint
            )
            if in_lane:
                problems.append(f"{name}: столб в полосе движения ({x}, {z})")

    problems.extend(walkable_problems(m))

    tiles = defaultdict(int)
    for light in m["lights"]:
        tiles[(int(light["pos"][0] // 25), int(light["pos"][2] // 25))] += 1
    worst = max(tiles.values()) if tiles else 0
    if worst > Map.LIGHT_PER_TILE:
        problems.append(f"на одну плитку земли приходится {worst} ламп")

    if len(m["districts"]) < 6:
        problems.append("районов меньше шести — упадёт тест пресета")
    if len(m["route_layers"]) < 8:
        problems.append("маршрутов меньше восьми — упадёт тест пресета")
    if len(m["skyline_beacons"]) < 10:
        problems.append("маяков меньше десяти — упадёт тест пресета")
    for beacon in m["skyline_beacons"]:
        if not 3.0 <= beacon["height"] <= 30.0:
            problems.append(f"маяк {beacon['id']}: высота вне 3..30")
    return problems


# Максимальная вертикальная ступень, которую персонаж переступает. Ноль с
# небольшим: `move_and_slide` без подъёма на ступеньку упирается в любую
# вертикальную грань, так что запас чисто на стыки плит.
MAX_STEP = 0.12


def surface_height(m: dict, x: float, z: float) -> float:
    """Высота твёрдой поверхности в точке — по чему игрок тут пойдёт."""
    top = 0.0
    for block in m["blocks"]:
        shape = block["shape"]
        if shape == "box":
            bx, by, bz = block["pos"]
            sx, sy, sz = block["size"]
            dx, dz = x - bx, z - bz
            if abs(block.get("rot", 0.0) - 45.0) < 0.1:
                # Повёрнутая на 45° плита: считаем в её собственных осях.
                dx, dz = (dx + dz) * 0.70710678, (dz - dx) * 0.70710678
            if abs(dx) <= sx / 2.0 and abs(dz) <= sz / 2.0:
                top = max(top, by + sy / 2.0)
        elif shape == "cylinder":
            bx, by, bz = block["pos"]
            if math.hypot(x - bx, z - bz) <= block["radius"]:
                top = max(top, by + block["height"])
        elif shape in ("ramp", "stairs"):
            fx, fy, fz = block["from"]
            tx, ty, tz = block["to"]
            run_x, run_z = tx - fx, tz - fz
            run = math.hypot(run_x, run_z)
            if run < 0.01:
                continue
            ux, uz = run_x / run, run_z / run
            along = (x - fx) * ux + (z - fz) * uz
            across = abs(-(x - fx) * uz + (z - fz) * ux)
            if 0.0 <= along <= run and across <= block.get("width", 1.0) / 2.0:
                top = max(top, fy + (ty - fy) * along / run)
    return top


def walkable_problems(m: dict) -> list[str]:
    """Пройти коридор от спавна до врат и убедиться, что путь непрерывен.

    Персонаж не поднимается на ступеньку, поэтому «дорога есть» означает: по
    какой-то полосе высота меняется плавно, без вертикальных уступов.
    """
    spawn_z = m["player_spawn"][2]
    gate_z = m["gate"][2]
    problems = []
    best_lane, best_gap = None, 1e9
    for lane in (-3.0, -1.5, 0.0, 1.5, 3.0):
        z = spawn_z
        previous = surface_height(m, lane, z)
        worst = 0.0
        where = z
        while z > gate_z + 2.0:
            z -= 0.4
            height = surface_height(m, lane, z)
            climb = height - previous
            if climb > worst:
                worst, where = climb, z
            previous = height
        if worst < best_gap:
            best_gap, best_lane = worst, (lane, where)
    if best_gap > MAX_STEP:
        lane, where = best_lane
        problems.append(
            f"дорога к вратам перекрыта: уступ {best_gap:.2f} м на x={lane}, z={where:.1f}"
        )
    return problems


def draw_plan(m: dict, path: str) -> None:
    """Верхний вид карты картинкой: композицию видно сразу, без движка."""
    from PIL import Image, ImageDraw  # локальный импорт: нужен только для --plan

    scale = 4
    size = int(GROUND) * scale
    img = Image.new("RGB", (size, size), (12, 8, 18))
    draw = ImageDraw.Draw(img)

    def px(x: float, z: float):
        return (x + GROUND / 2) * scale, (GROUND / 2 - z) * scale

    for _, x0, z0, x1, z1 in STREETS:
        draw.rectangle([px(x0, z1), px(x1, z0)], fill=(30, 22, 42))
    for b in m["buildings"]:
        cx, cz = b["pos"]
        w, h, d = b["size"]
        shade = int(60 + min(h, 40) * 4)
        draw.rectangle(
            [px(cx - w / 2, cz + d / 2), px(cx + w / 2, cz - d / 2)],
            fill=(shade, int(shade * 0.5), int(shade * 0.8)),
            outline=(220, 120, 200),
        )
    for route in m["route_layers"]:
        pts = [px(p[0], p[2]) for p in route["points"]]
        draw.line(pts, fill=(255, 110, 190), width=3)
    for light in m["lights"]:
        x, y, z = light["pos"]
        cx, cz = px(x, z)
        draw.ellipse([cx - 3, cz - 3, cx + 3, cz + 3], fill=(255, 230, 140))
    for d in m["districts"]:
        cx, cz = px(d["center"][0], d["center"][1])
        r = d["radius"] * scale
        draw.ellipse([cx - r, cz - r, cx + r, cz + r],
                     outline=tuple(int(c * 255) for c in d["color"]))
        draw.text((cx - 20, cz - 6), d["id"], fill=(255, 255, 255))
    for npc in json.load(io.open(NPCS_PATH, encoding="utf-8")):
        pos = npc.get("pos")
        if pos:
            cx, cz = px(float(pos[0]), float(pos[1]))
            draw.ellipse([cx - 5, cz - 5, cx + 5, cz + 5], fill=(120, 255, 180))
    cx, cz = px(GATE[0], GATE[2])
    draw.ellipse([cx - 7, cz - 7, cx + 7, cz + 7], fill=(255, 60, 90))
    cx, cz = px(PLAYER_SPAWN[0], PLAYER_SPAWN[2])
    draw.ellipse([cx - 7, cz - 7, cx + 7, cz + 7], fill=(80, 200, 255))
    img.save(path)


NUMBER_ROW = re.compile(r"\[\s*(-?[\d.eE+]+(?:,\s*-?[\d.eE+]+)*)\s*\]", re.S)


def compact(text: str) -> str:
    """Свернуть массивы чисел в строку: иначе координаты не прочесть глазами."""
    def one_line(match: "re.Match[str]") -> str:
        parts = [p.strip() for p in match.group(1).split(",")]
        return "[" + ", ".join(parts) + "]"

    return NUMBER_ROW.sub(one_line, text)


def main() -> int:
    parser = argparse.ArgumentParser(description="сборка карты хаба")
    parser.add_argument("--check", action="store_true", help="не писать, только сверить")
    parser.add_argument("--plan", metavar="PNG", help="сохранить верхний вид")
    args = parser.parse_args()

    previous = json.load(io.open(MAP_PATH, encoding="utf-8"))
    # Расстановка врагов и предметов — это баланс, а не композиция: переносим
    # её как есть, чтобы генератор не трогал геймплей.
    result = build(previous.get("spawns", {}))

    problems = validate(result, previous.get("spawns", {}))
    for problem in problems:
        print("!!", problem)

    text = compact(json.dumps(result, ensure_ascii=False, indent=1)) + "\n"

    if args.plan:
        draw_plan(result, args.plan)
        print("план:", args.plan)

    if args.check:
        current = io.open(MAP_PATH, encoding="utf-8").read()
        if current != text:
            print("hub.json отличается от генератора: пересобери его")
            return 1
        print("hub.json совпадает с генератором")
        return 1 if problems else 0

    io.open(MAP_PATH, "w", encoding="utf-8", newline="\n").write(text)
    print(
        f"hub.json: домов {len(result['buildings'])}, блоков {len(result['blocks'])}, "
        f"неона {len(result['glows'])}, ламп {len(result['lights'])}, "
        f"пропсов {len(result['props'])}, маяков {len(result['skyline_beacons'])}"
    )
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
