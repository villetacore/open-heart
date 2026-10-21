# Инвентаризация проекта

Создано `tools/audit_project.py`. Счётчики получены из JSON на диске; runtime предпочитает RON. Это не проверка поведения игры.

## Исходники

| Область | Файлы | Строки |
|---|---:|---:|
| `core/src` | 54 | 14544 |
| `client/src` | 46 | 20094 |
| `terminal/src` | 10 | 2900 |
| `server` | 37 | 6466 |
| `game` | 14 | 2669 |

## Пресет arena

| Набор | Записей |
|---|---:|
| `classes` | 3 |
| `enemies.enemies` | 14 |
| `items.items` | 18 |
| `level.spawn_enemies` | 0 |
| `level.spawn_items` | 0 |
| `level.spawn_ammo` | 0 |
| `level.spawn_weapons` | 0 |
| `npcs` | 0 |
| `perks` | 18 |
| `quests` | 0 |
| `synergies` | 8 |
| `weapons` | 8 |

### Цепочки заданий

| Цепочка | Этапов |
|---|---:|

## Пресет core

| Набор | Записей |
|---|---:|
| `abilities` | 8 |
| `affixes` | 5 |
| `classes` | 3 |
| `dialogues` | 154 |
| `dungeon.themes` | 5 |
| `dungeon.pools` | 4 |
| `dungeon.room_archetypes` | 16 |
| `enemies.enemies` | 21 |
| `items.items` | 21 |
| `level.spawn_enemies` | 25 |
| `level.spawn_items` | 22 |
| `level.spawn_ammo` | 12 |
| `level.spawn_weapons` | 5 |
| `loot.room_items` | 8 |
| `loot.kill_drops` | 5 |
| `npcs` | 21 |
| `perks` | 21 |
| `player_abilities` | 6 |
| `quests` | 41 |
| `recipes.recipes` | 8 |
| `statuses` | 5 |
| `synergies` | 11 |
| `weapon_mods` | 16 |
| `weapons` | 8 |

### Цепочки заданий

| Цепочка | Этапов |
|---|---:|
| Архивный сигнал | 3 |
| Глубинный дозор | 7 |
| Линия снабжения | 3 |
| Нити квартала | 3 |
| Огни площади | 1 |
| Охота квартала | 4 |
| Почта квартала | 1 |
| Реликвии квартала | 3 |
| Свет для общего стола | 4 |
| Соседи: Ив | 2 |
| Соседи: Иво | 2 |
| Соседи: Люсьен | 2 |
| Соседи: Ноэль | 2 |
| Соседи: Эмиль | 2 |
| Соседи: Эш | 2 |

## Статические ссылки ресурсов

Отсутствующих файлов по `path="res://..."` в .tres/.tscn: **0**.

Полный машинный отчёт: [project_inventory.json](project_inventory.json). Проверка не охватывает вычисляемые пути Rust/GDScript, область атласа, визуальное качество и экспорт.
