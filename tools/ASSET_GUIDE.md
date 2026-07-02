# OpenHeart — Гайд по генерации и обработке ассетов

## Установка скрипта
```
pip install Pillow
```

---

## Форматы (что должно получиться)

| Тип          | Итоговый PNG                     | Размер      | Описание                          |
|--------------|----------------------------------|-------------|-----------------------------------|
| Персонаж/NPC | `sprites/characters/npc_*.png`   | 512 × 256   | 4 кадра по 128×256 (idle×2, walk×2) |
| Враг         | `sprites/characters/enemy_*.png` | 512 × 256   | то же                             |
| Предмет      | `sprites/items/item_*.png`       | 128 × 64    | 2 кадра по 64×64 (idle, glow)     |
| Текстура     | `textures/wall_*.png` и т.д.     | 512 × 512   | бесшовная тайловая                |
| Оружие       | `sprites/weapon/weapon_pistol.png`| 384 × 256  | 3 кадра по 128×256 (idle, recoil1, recoil2) |

---

## 1. Спрайты персонажей (NPC и враги)

### Что генерировать в ChatGPT
Для каждого персонажа нужно **одно изображение** — анонимный скрипт сам сделает из него 4 кадра анимации.

### Нужные NPC (8 штук)
| ID файла          | Персонаж    | Описание для промпта |
|-------------------|-------------|----------------------|
| `npc_vale`        | Ms. Вейл    | молодая женщина ~27 лет, строгий серый пиджак, тёмные волосы, уверенный взгляд, лёгкая улыбка |
| `npc_victor`      | Виктор      | парень ~18 лет, растрёпанные волосы, широкая ухмылка, неформальная одежда, дружелюбный вид |
| `npc_elena`       | Елена       | девушка ~17 лет, строгий взгляд, очки, стопка книг, синяя школьная форма |
| `npc_sofia`       | София       | блондинка ~17 лет, безупречный вид, высокомерный взгляд, дорогая одежда |
| `npc_guard`       | Охранник    | крупный мужчина, форма охранника тёмного цвета, суровое лицо, рука на поясе |
| `npc_merchant`    | Торговец    | полноватый мужчина средних лет, хитрые глаза, яркая жилетка, широкая улыбка |
| `npc_scientist`   | Учёный      | пожилой мужчина в белом халате, взволнованный вид, очки на лбу, лохматые волосы |
| `npc_stranger`    | Незнакомец  | фигура в тёмном капюшоне, лицо полускрыто, светлые глаза, загадочный вид |

### Нужные враги (6 штук)
| ID файла          | Враг      | Описание для промпта |
|-------------------|-----------|----------------------|
| `enemy_grunt`     | Grunt     | боевик в красном, агрессивный, тёмная маска, мускулистый |
| `enemy_fast`      | Chaser    | худощавый враг в зелёном, быстрый, хищная стойка |
| `enemy_heavy`     | Heavy     | огромный враг в фиолетовых доспехах, медленный, угрожающий |
| `enemy_brute`     | Brute     | гигант в оранжевом, разрушительный, массивные кулаки |
| `enemy_sniper`    | Sniper    | тонкий враг в жёлтом, скрытный, дальнобойная стойка |
| `enemy_cultist`   | Cultist   | культист в синих одеяниях, тёмные рисунки на коже, безумный взгляд |

### Промпт-шаблон для ChatGPT (DALL-E)
```
Game character sprite, anime style, full body, front-facing pose,
[ОПИСАНИЕ_ПЕРСОНАЖА],
pure black background, no shadow, clean edges,
vertical portrait composition, the character takes up 80% of the frame,
clear silhouette, game-ready 2D sprite
```

**Примеры:**
- Vale: `Game character sprite, anime style, full body, front-facing pose, young woman ~27 years old, strict grey blazer, dark hair tied back, confident slight smile, purple-pink color palette, pure black background, no shadow, clean edges, vertical portrait composition, the character takes up 80% of the frame, clear silhouette, game-ready 2D sprite`
- Grunt: `Game character sprite, anime style, full body, front-facing pose, aggressive soldier in red tactical armor, dark face mask, muscular build, threatening stance, red and black color palette, pure black background, no shadow, clean edges, vertical portrait composition, clear silhouette, game-ready 2D sprite`

### Обработка после генерации
```bash
# Один файл:
python tools/process_sprites.py character generated/vale_raw.png npc_vale
python tools/process_sprites.py character generated/grunt_raw.png enemy_grunt

# Папка с файлами (имя файла = id):
python tools/process_sprites.py batch_chars generated/characters/
```

### Если у тебя 2-4 отдельных кадра для одного персонажа
```bash
python tools/process_sprites.py character_frames idle1.png idle2.png walk1.png walk2.png npc_vale
```

---

## 2. Текстуры окружения (8 штук)

### Нужные файлы
| Файл              | Где используется     |
|-------------------|----------------------|
| `wall_main`       | Центральный зал      |
| `wall_archive`    | Архив                |
| `wall_boss`       | Тронный зал          |
| `wall_market`     | Восточный рынок      |
| `wall_lab`        | Западная лаборатория |
| `wall_arena`      | Южная арена          |
| `floor_main`      | Общий пол            |
| `ceiling_dark`    | Потолок              |

### Промпт-шаблон
```
Seamless tileable texture, [ОПИСАНИЕ], top-down flat view,
dark gothic style, game texture, no perspective distortion,
512x512, photorealistic or stylized, high detail
```

**Примеры:**
- `wall_main`: `Seamless tileable stone wall texture, dark gothic style, carved stone blocks, faint purple/pink lighting hints, game texture, dark and moody`
- `wall_archive`: `Seamless tileable dark bookshelf texture or old stone wall with shelving patterns, blue-grey tones, library atmosphere, game texture`
- `wall_boss`: `Seamless tileable ancient dark stone texture with red ritual markings, throne room atmosphere, ominous, dark red and black`
- `wall_market`: `Seamless tileable warm stone wall with market stall decorations, gold and amber tones, fantasy bazaar, game texture`
- `wall_lab`: `Seamless tileable metal panel wall texture, sci-fi laboratory, green glowing vents, clean but dark`
- `wall_arena`: `Seamless tileable rough stone arena wall texture, battle-worn, orange and amber torchlight tones`
- `floor_main`: `Seamless tileable dark stone floor texture, gothic fantasy, subtle geometric pattern, grey-purple`
- `ceiling_dark`: `Seamless tileable very dark stone ceiling texture, almost black, subtle cracks`

### Обработка
```bash
python tools/process_sprites.py texture generated/wall_main_raw.png wall_main
python tools/process_sprites.py texture generated/floor_raw.png floor_main
# и т.д. для каждой текстуры
```

---

## 3. Спрайты предметов (7 штук)

### Нужные файлы
| Файл                | Предмет         |
|---------------------|-----------------|
| `item_medkit`       | Аптечка         |
| `item_key`          | Серебряный ключ |
| `item_gold`         | Монета/золото   |
| `item_armor`        | Бронепластина   |
| `item_energy_drink` | Энергетик       |
| `item_potion`       | Зелье           |
| `item_ruby`         | Рубин           |

### Промпт-шаблон
```
Pixel art game item icon, [ОПИСАНИЕ ПРЕДМЕТА], isolated on white background,
simple clean design, fantasy RPG style, small icon, bright readable silhouette
```

**Примеры:**
- `item_medkit`: `Pixel art game item icon, red and white first aid kit with cross symbol, isolated on white background, simple clean design, fantasy RPG style`
- `item_key`: `Pixel art game item icon, silver ornate skeleton key with gems, isolated on white background, shiny metallic, fantasy RPG style`
- `item_gold`: `Pixel art game item icon, golden coin stack with shine effect, isolated on white background, bright gold color, fantasy RPG style`
- `item_ruby`: `Pixel art game item icon, large faceted red ruby gemstone, glowing, isolated on white background, precious jewel, fantasy RPG style`

### Обработка
```bash
python tools/process_sprites.py item generated/medkit_raw.png item_medkit
python tools/process_sprites.py item generated/key_raw.png item_key
```

---

## 4. Оружие (HUD-спрайт)

### Что нужно
Один спрайт пистолета в позиции от первого лица — оружие в нижнем правом углу экрана, как в Doom.

### Промпт
```
First-person view game weapon sprite, sci-fi pistol held in right hand,
black background, viewed from first-person perspective as if holding it,
bottom-right orientation, dark metallic gun, anime stylized,
no HUD elements, isolated weapon only, clean edges
```

### Обработка
```bash
python tools/process_sprites.py weapon generated/pistol_raw.png
```

---

## Полный пайплайн (чеклист)

```
□ Генерируешь в ChatGPT (1 запрос = 1 изображение)
□ Сохраняешь в папку tools/generated/
□ Запускаешь python tools/process_sprites.py <команда> ...
□ Скрипт кладёт файлы в godot/assets/sprites/ и godot/assets/textures/
□ Перезапускаешь игру — спрайты подхватываются автоматически
```

---

## Советы по генерации в ChatGPT

1. **Фон** — всегда проси `pure black background` или `white background`. Скрипт уберёт его автоматически.
2. **Размер** — ChatGPT генерирует 1024×1024 или 1792×1024. Скрипт сам отресайзит.
3. **Стиль** — используй `anime style` или `2D game sprite` для единообразия.
4. **Полный рост** — для персонажей обязательно `full body, front-facing pose` — иначе получишь только лицо.
5. **Итерации** — если первый результат не нравится, добавь: `more detailed, cleaner lines, bolder colors`.
6. **Для спрайтшитов из 4 кадров** — можно попросить ChatGPT: `4 animation frames in a row, same character, slight movement variation, sprite sheet layout, horizontal strip`.

---

## Структура файлов после обработки

```
godot/assets/
├── sprites/
│   ├── characters/
│   │   ├── npc_vale.png       (512×256)
│   │   ├── npc_victor.png
│   │   ├── npc_elena.png
│   │   ├── npc_sofia.png
│   │   ├── npc_guard.png
│   │   ├── npc_merchant.png
│   │   ├── npc_scientist.png
│   │   ├── npc_stranger.png
│   │   ├── enemy_grunt.png    (512×256)
│   │   ├── enemy_fast.png
│   │   ├── enemy_heavy.png
│   │   ├── enemy_brute.png
│   │   ├── enemy_sniper.png
│   │   └── enemy_cultist.png
│   ├── items/
│   │   ├── item_medkit.png    (128×64)
│   │   ├── item_key.png
│   │   ├── item_gold.png
│   │   ├── item_armor.png
│   │   ├── item_energy_drink.png
│   │   ├── item_potion.png
│   │   └── item_ruby.png
│   └── weapon/
│       └── weapon_pistol.png  (384×256)
└── textures/
    ├── wall_main.png          (512×512)
    ├── wall_archive.png
    ├── wall_boss.png
    ├── wall_market.png
    ├── wall_lab.png
    ├── wall_arena.png
    ├── floor_main.png
    └── ceiling_dark.png
```
