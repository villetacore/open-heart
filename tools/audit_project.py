"""Generate a factual repository/content inventory without changing game data.

Run from any directory: python tools/audit_project.py
This is a disk inventory, not a claim of runtime or release readiness.
"""
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/_generated'

def relative(path):
    return path.relative_to(ROOT).as_posix()

def read_json(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def main():
    OUT.mkdir(parents=True, exist_ok=True)
    data = {'scope': 'Disk inventory, not runtime verification', 'source': {}, 'presets': {}, 'missing_resource_files': []}
    for folder, extension in [('core/src', '.rs'), ('client/src', '.rs'), ('terminal/src', '.rs'),
                              ('server', '.go'), ('game', '.gd')]:
        files = sorted((ROOT / folder).rglob('*' + extension))
        files = [p for p in files if '.godot' not in p.parts]
        rows = [dict(path=relative(p), lines=len(p.read_text(encoding='utf-8-sig').splitlines())) for p in files]
        data['source'][folder] = dict(files=len(rows), lines=sum(p['lines'] for p in rows), largest=sorted(rows,key=lambda p:p['lines'],reverse=True)[:8])
    for preset in sorted((ROOT/'game/presets').iterdir()):
        if not preset.is_dir():
            continue
        counts = {}
        for path in sorted(preset.glob('*.json')):
            value = read_json(path)
            if isinstance(value, list):
                counts[path.stem] = len(value)
            elif isinstance(value, dict):
                for key, entries in value.items():
                    if isinstance(entries, list):
                        counts[path.stem + '.' + key] = len(entries)
        item = dict(counts=counts, maps=[relative(p) for p in sorted((preset/'maps').glob('*.json'))])
        npcs_file, quests_file = preset/'npcs.json', preset/'quests.json'
        if npcs_file.exists():
            item['npcs'] = [dict(id=n['id'],name=n.get('name_ru',''),sprite=n.get('sprite',''),scene=n.get('scene','')) for n in read_json(npcs_file)]
        if quests_file.exists():
            quests = read_json(quests_file)
            item['quest_types'] = dict(sorted(Counter(q['kind'] for q in quests).items()))
            item['chains'] = dict(sorted(Counter(q.get('chain_ru','') for q in quests).items()))
            item['world_changes'] = [dict(quest=q['id'],pattern=q['world_change']['pattern']) for q in quests if q.get('world_change')]
        data['presets'][preset.name] = item
    assets = ROOT/'game/assets'
    data['assets'] = dict(sorted(Counter(p.suffix.lower() for p in assets.rglob('*') if p.is_file() and p.suffix != '.import').items()))
    # Only static scene/resource references. Dynamic paths in source need separate review.
    for extension in ('.tres','.tscn'):
        for path in sorted((ROOT/'game').rglob('*'+extension)):
            if '.godot' in path.parts:
                continue
            for ref in re.findall(r'path="(res://[^\"]+)"', path.read_text(encoding='utf-8-sig')):
                if not (ROOT/'game'/ref[6:]).is_file():
                    data['missing_resource_files'].append(dict(source=relative(path),resource=ref))
    (OUT/'project_inventory.json').write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    lines = ['# Инвентаризация проекта', '', 'Создано `tools/audit_project.py`. Счётчики получены из JSON на диске; runtime предпочитает RON. Это не проверка поведения игры.', '', '## Исходники', '', '| Область | Файлы | Строки |', '|---|---:|---:|']
    for folder, info in data['source'].items():
        lines.append(f"| `{folder}` | {info['files']} | {info['lines']} |")
    for preset, info in data['presets'].items():
        lines += ['', f'## Пресет {preset}', '', '| Набор | Записей |', '|---|---:|']
        lines += [f'| `{key}` | {count} |' for key,count in info['counts'].items()]
        lines += ['', '### Цепочки заданий', '', '| Цепочка | Этапов |', '|---|---:|']
        lines += [f'| {key} | {count} |' for key,count in info.get('chains',{}).items()]
    lines += ['', '## Статические ссылки ресурсов', '', f"Отсутствующих файлов по `path=\"res://...\"` в .tres/.tscn: **{len(data['missing_resource_files'])}**.", '', 'Полный машинный отчёт: [project_inventory.json](project_inventory.json). Проверка не охватывает вычисляемые пути Rust/GDScript, область атласа, визуальное качество и экспорт.']
    (OUT/'INVENTORY.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
    print(json.dumps(data,ensure_ascii=False,indent=2))

if __name__ == '__main__':
    main()
