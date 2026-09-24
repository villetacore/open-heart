# OpenHeart Preset Format v2

Preset v2 uses `preset.toml` for metadata, RON for authored gameplay content, and JSON
for generated maps. Existing JSON content remains supported.

## File priority

For content named `weapons`, the loader checks:

1. `weapons.ron`
2. `weapons.json`
3. `weapons.toml`

This allows gradual migration without breaking existing presets.

## Manifest

```toml
schema_version = 2
id = "my_campaign"
name_ru = "Моя кампания"
desc_ru = "Описание"
author = "Author"
version = 1
enabled = true
entry_map = "hub"
tags = ["campaign"]
```

## Recommended layout

```text
presets/my_campaign/
  preset.toml
  weapons.ron
  enemies.ron
  quests.ron
  items.ron
  maps/
    hub.json
```

RON files use the same field names as their legacy JSON equivalents. JSON remains the
recommended map format because maps are generated and edited by tools.

Convert an existing preset:

```powershell
# from the repository root
cargo run -p openheart --bin preset_migrate -- game/presets/core
```

The converter writes sibling `.ron` files and keeps the JSON sources intact for rollback.

## Compatibility

- Presets without `schema_version` are treated as v1.
- `preset.json` remains a supported fallback.
- A `.ron` file overrides a sibling `.json` file.
- Invalid content falls back to the embedded `core` data when that category requires it.
