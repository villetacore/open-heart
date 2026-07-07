#!/usr/bin/env python3
"""
OpenHeart AI Texture Generator — клиент к серверу генерации изображений.

Генерирует изображение по промпту (Stable Diffusion: AUTOMATIC1111 или ComfyUI),
сохраняет сырой результат в tools/generated/ и прогоняет через постобработку
tools/process_sprites.py (удаление фона, ресайз, сборка кадров, раскладка по
папкам godot/assets/). Игра подхватывает новые PNG при следующем запуске —
импорт в Godot не нужен.

Настройки сервера: tools/aigen.json (url, backend, model, steps...).
Шаблоны промптов:  tools/aigen_templates.json (по типам ассетов).

Использование:
  python aigen.py <тип> <id> "<описание>"
      тип: character | texture | item | weapon
      id:  npc_vale / enemy_grunt / wall_main / item_medkit / weapon (см. id_hint)

  python aigen.py character enemy_pyro "cultist in burning red robes, flame patterns"
  python aigen.py texture wall_lab "sci-fi metal panels with green glowing vents"

Флаги:
  --no-template   описание = готовый промпт (шаблон типа не подставляется)
  --raw-only      только сгенерировать raw в tools/generated/, без постобработки
  --input FILE    пропустить генерацию, постобработать существующий файл
  --seed N        зафиксировать сид генерации

Последняя строка вывода машиночитаема (для редактора):
  OK <путь к итоговому PNG>   |   ERR <причина>
"""

import base64
import json
import random
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
GENERATED  = SCRIPT_DIR / "generated"
CONFIG     = SCRIPT_DIR / "aigen.json"
TEMPLATES  = SCRIPT_DIR / "aigen_templates.json"

# консоль Windows может быть не в UTF-8
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def fail(msg: str) -> None:
    print(f"ERR {msg}")
    sys.exit(1)


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(f"нет файла {path}")
    except json.JSONDecodeError as e:
        fail(f"{path.name}: битый JSON ({e})")


def http_json(url: str, payload: dict | None, timeout: float):
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(
        url, data=data,
        headers={"Content-Type": "application/json"},
        method="POST" if data else "GET",
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read())


# ── Бэкенды ───────────────────────────────────────────────────────────────────

def gen_a1111(cfg: dict, prompt: str, width: int, height: int, seed: int) -> bytes:
    """AUTOMATIC1111 WebUI: один POST /sdapi/v1/txt2img → base64 PNG."""
    payload = {
        "prompt": prompt,
        "negative_prompt": cfg.get("negative", ""),
        "steps": cfg.get("steps", 28),
        "cfg_scale": cfg.get("cfg_scale", 7.0),
        "sampler_name": cfg.get("sampler", "Euler a"),
        "width": width,
        "height": height,
        "seed": seed,
    }
    if cfg.get("model"):
        payload["override_settings"] = {"sd_model_checkpoint": cfg["model"]}
    out = http_json(f"{cfg['url'].rstrip('/')}/sdapi/v1/txt2img",
                    payload, cfg.get("timeout_sec", 300))
    images = out.get("images") or []
    if not images:
        fail("сервер A1111 вернул пустой ответ (нет images)")
    return base64.b64decode(images[0])


def gen_comfyui(cfg: dict, prompt: str, width: int, height: int, seed: int) -> bytes:
    """ComfyUI: POST /prompt (базовый txt2img-граф) → поллинг /history → GET /view."""
    ccfg = cfg.get("comfyui", {})
    checkpoint = ccfg.get("checkpoint") or cfg.get("model")
    if not checkpoint:
        fail("для comfyui укажи checkpoint в aigen.json (comfyui.checkpoint)")
    url = cfg["url"].rstrip("/")
    graph = {
        "1": {"class_type": "CheckpointLoaderSimple",
              "inputs": {"ckpt_name": checkpoint}},
        "2": {"class_type": "CLIPTextEncode",
              "inputs": {"clip": ["1", 1], "text": prompt}},
        "3": {"class_type": "CLIPTextEncode",
              "inputs": {"clip": ["1", 1], "text": cfg.get("negative", "")}},
        "4": {"class_type": "EmptyLatentImage",
              "inputs": {"width": width, "height": height, "batch_size": 1}},
        "5": {"class_type": "KSampler",
              "inputs": {"model": ["1", 0], "positive": ["2", 0], "negative": ["3", 0],
                         "latent_image": ["4", 0], "seed": seed,
                         "steps": cfg.get("steps", 28), "cfg": cfg.get("cfg_scale", 7.0),
                         "sampler_name": "euler_ancestral", "scheduler": "normal",
                         "denoise": 1.0}},
        "6": {"class_type": "VAEDecode",
              "inputs": {"samples": ["5", 0], "vae": ["1", 2]}},
        "7": {"class_type": "SaveImage",
              "inputs": {"images": ["6", 0], "filename_prefix": "openheart_aigen"}},
    }
    queued = http_json(f"{url}/prompt", {"prompt": graph}, cfg.get("timeout_sec", 300))
    pid = queued.get("prompt_id")
    if not pid:
        fail(f"ComfyUI не принял граф: {queued}")

    deadline = time.time() + cfg.get("timeout_sec", 300)
    poll = ccfg.get("poll_interval_sec", 1.0)
    while time.time() < deadline:
        hist = http_json(f"{url}/history/{pid}", None, 30).get(pid)
        if hist:
            for node_out in hist.get("outputs", {}).values():
                for img in node_out.get("images", []):
                    q = urllib.parse.urlencode({
                        "filename": img["filename"],
                        "subfolder": img.get("subfolder", ""),
                        "type": img.get("type", "output"),
                    })
                    with urllib.request.urlopen(f"{url}/view?{q}", timeout=60) as r:
                        return r.read()
            fail("ComfyUI завершил задачу без изображений")
        time.sleep(poll)
    fail("таймаут ожидания ComfyUI")


# ── Постобработка и раскладка ────────────────────────────────────────────────

def _import_ps():
    """Ленивый импорт постобработки: --raw-only работает даже без Pillow."""
    sys.path.insert(0, str(SCRIPT_DIR))
    try:
        import process_sprites as ps
    except ImportError as e:
        fail(f"постобработка недоступна ({e}) — установи Pillow: pip install Pillow")
    return ps


def postprocess(asset_type: str, asset_id: str, raw_path: Path) -> Path:
    """Прогнать raw через process_sprites.py; вернуть путь итогового файла."""
    ps = _import_ps()
    if asset_type == "character":
        ps.process_character(str(raw_path), asset_id)
        return ps.OUT_CHARS / f"{asset_id}.png"
    if asset_type == "item":
        ps.process_item(str(raw_path), asset_id)
        return ps.OUT_ITEMS / f"{asset_id}.png"
    if asset_type == "weapon":
        ps.process_weapon(str(raw_path))
        return ps.OUT_WEAPON / "weapon_pistol.png"
    # texture: process_texture пишет в textures/, но движок ищет часть имён
    # в подпапках (см. rust/src/map.rs::tex_path) — переносим по префиксу.
    ps.process_texture(str(raw_path), asset_id)
    out = ps.OUT_TEXTURES / f"{asset_id}.png"
    sub = None
    if asset_id.startswith(("dtile_", "liquid_")):
        sub = ps.OUT_TEXTURES / "dungeon"
    elif asset_id.startswith("sky_"):
        sub = ps.OUT_TEXTURES / "sky"
    if sub:
        sub.mkdir(parents=True, exist_ok=True)
        target = sub / out.name
        out.replace(target)
        print(f"[OK] перенесено по префиксу -> {target}")
        return target
    return out


# ── main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    args = [a for a in sys.argv[1:]]
    flags = {"--no-template": False, "--raw-only": False}
    seed = None
    input_file = None
    positional = []
    i = 0
    while i < len(args):
        a = args[i]
        if a in flags:
            flags[a] = True
        elif a in ("--seed", "--input"):
            i += 1
            if i >= len(args):
                fail(f"флагу {a} нужно значение")
            if a == "--seed":
                try:
                    seed = int(args[i])
                except ValueError:
                    fail(f"--seed ожидает целое число, получено '{args[i]}'")
            else:
                input_file = Path(args[i])
        else:
            positional.append(a)
        i += 1

    if len(positional) < 2 or (len(positional) < 3 and input_file is None):
        print(__doc__)
        fail("нужно: <тип> <id> \"<описание>\" (или --input file без описания)")

    asset_type, asset_id = positional[0], positional[1]
    # не заключённое в кавычки описание из нескольких слов тоже принимаем
    desc = " ".join(positional[2:])

    templates = load_json(TEMPLATES)
    if asset_type not in templates or asset_type.startswith("_"):
        fail(f"неизвестный тип '{asset_type}' — есть: "
             + ", ".join(k for k in templates if not k.startswith("_")))
    tpl = templates[asset_type]

    if input_file is not None:
        # только постобработка готового файла
        if not input_file.exists():
            fail(f"нет входного файла {input_file}")
        final = postprocess(asset_type, asset_id, input_file)
        print(f"OK {final}")
        return

    cfg = load_json(CONFIG)
    prompt = desc if flags["--no-template"] else tpl["prompt"].format(desc=desc)
    width, height = tpl.get("width", 1024), tpl.get("height", 1024)
    if seed is None:
        seed = random.randrange(2**31)

    print(f"[gen] {cfg.get('backend', 'a1111')} @ {cfg.get('url')}")
    print(f"[gen] {asset_type}/{asset_id} {width}x{height} seed={seed}")
    print(f"[gen] prompt: {prompt}")

    backend = cfg.get("backend", "a1111")
    try:
        if backend == "a1111":
            png = gen_a1111(cfg, prompt, width, height, seed)
        elif backend == "comfyui":
            png = gen_comfyui(cfg, prompt, width, height, seed)
        else:
            fail(f"неизвестный backend '{backend}' (a1111 | comfyui)")
    except urllib.error.URLError as e:
        fail(f"сервер недоступен ({cfg.get('url')}): {e}")
    except TimeoutError:
        fail(f"таймаут запроса к {cfg.get('url')}")

    GENERATED.mkdir(parents=True, exist_ok=True)
    raw_path = GENERATED / f"{asset_id}_raw.png"
    raw_path.write_bytes(png)
    print(f"[gen] raw -> {raw_path}")

    if flags["--raw-only"]:
        print(f"OK {raw_path}")
        return

    final = postprocess(asset_type, asset_id, raw_path)
    if asset_type == "character" and asset_id.startswith("enemy_"):
        print(f"[hint] в enemies.json поле sprite = \"{asset_id[len('enemy_'):]}\"")
    print(f"OK {final}")


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception as e:  # контракт: последняя строка вывода — всегда OK/ERR
        fail(f"{type(e).__name__}: {e}")
