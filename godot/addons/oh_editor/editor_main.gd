@tool
extends Control
## Главная панель «OpenHeart»: редактор пресетов игры.
##
## Слева — категория и список записей, справа — форма полей по схеме.
## Всё редактирование идёт в память (Dictionary/Array из JSON) и пишется на диск
## кнопкой «Сохранить пресет». Схемы описывают каждый тип контента декларативно,
## так что новая категория = несколько строк в SCHEMAS.

# ── Схемы контента ────────────────────────────────────────────────────────────
# type: str | text (многострочный) | float | int | bool | enum | json
#       vec2 | vec3 (спинбоксы) | color ([r,g,b] через палитру)
#       dyn_enum — выпадающий список из ДАННЫХ пресета (source: npcs | enemies |
#       items | targets | quests | weapons | scenes | enemy_sprites)
# json — вложенные структуры (массивы/объекты) как текст c проверкой синтаксиса.
# Файл "maps/*" означает «выбранная карта» (переключатель в верхней панели).
# "single": true — файл-одиночная запись (preset.json), без CRUD.

const SCHEMAS := {
	"Оружие": {
		"file": "weapons.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "name_ru", "type": "str"},
			{"key": "damage", "type": "float"},
			{"key": "dmg_type", "type": "enum", "options": ["physical", "fire", "energy", "void"]},
			{"key": "cooldown", "type": "float"}, {"key": "range", "type": "float"},
			{"key": "auto", "type": "bool"},
			{"key": "fire", "type": "json"}, {"key": "ammo", "type": "json"},
			{"key": "sheet", "type": "str"}, {"key": "frame_h", "type": "float"},
			{"key": "idle_frames", "type": "json"}, {"key": "fire_frames", "type": "json"},
			{"key": "fire_fps", "type": "float"},
		],
	},
	"Классы": {
		"file": "classes.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "name_ru", "type": "str"},
			{"key": "role_ru", "type": "str"}, {"key": "desc_ru", "type": "text"},
			{"key": "base_hp", "type": "float"}, {"key": "speed", "type": "float"},
			{"key": "dmg_mult", "type": "float"},
			{"key": "start_weapons", "type": "json"}, {"key": "start_ammo", "type": "json"},
			{"key": "specs", "type": "json"},
		],
	},
	"Перки": {
		"file": "perks.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"},
			{"key": "branch", "type": "enum", "options": ["survival", "offense", "utility"]},
			{"key": "tier", "type": "int"}, {"key": "max_ranks", "type": "int"},
			{"key": "cost", "type": "int"}, {"key": "requires", "type": "json"},
			{"key": "name_ru", "type": "str"}, {"key": "desc_ru", "type": "text"},
			{"key": "effects", "type": "json"},
		],
	},
	"Синергии": {
		"file": "synergies.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "needs", "type": "json"},
			{"key": "name_ru", "type": "str"}, {"key": "desc_ru", "type": "text"},
			{"key": "effects", "type": "json"},
		],
	},
	"Враги": {
		"file": "enemies.json", "root": ["enemies"],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "name", "type": "str"},
			{"key": "hp", "type": "float"}, {"key": "speed", "type": "float"},
			{"key": "attack_damage", "type": "float"}, {"key": "attack_range", "type": "float"},
			{"key": "attack_cooldown", "type": "float"}, {"key": "chase_range", "type": "float"},
			{"key": "patrol_radius", "type": "float"}, {"key": "xp", "type": "float"},
			{"key": "sprite", "type": "dyn_enum", "source": "enemy_sprites"},
			{"key": "scale", "type": "float"},
			{"key": "color_r", "type": "float"}, {"key": "color_g", "type": "float"},
			{"key": "color_b", "type": "float"},
			{"key": "resist", "type": "json"},
		],
	},
	"Предметы": {
		"file": "items.json", "root": ["items"],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "name_ru", "type": "str"},
			{"key": "name_en", "type": "str"}, {"key": "desc_ru", "type": "text"},
			{"key": "desc_en", "type": "str"}, {"key": "value", "type": "int"},
			{"key": "category", "type": "enum", "options": ["consumable", "currency", "key"]},
			{"key": "heal", "type": "json"},
			{"key": "color_r", "type": "float"}, {"key": "color_g", "type": "float"},
			{"key": "color_b", "type": "float"},
		],
	},
	"NPC": {
		"file": "npcs.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "name_ru", "type": "str"},
			{"key": "sprite", "type": "str"}, {"key": "pos", "type": "vec2"},
			{"key": "color", "type": "json"},
			{"key": "scene", "type": "dyn_enum", "source": "scenes", "nullable": true},
			{"key": "quest", "type": "dyn_enum", "source": "quests", "nullable": true},
		],
	},
	"Квесты": {
		"file": "quests.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "title_ru", "type": "str"},
			{"key": "desc_ru", "type": "text"},
			{"key": "giver", "type": "dyn_enum", "source": "npcs"},
			{"key": "kind", "type": "enum", "options": ["kill", "collect", "clear_dungeon"]},
			{"key": "target", "type": "dyn_enum", "source": "targets"},
			{"key": "count", "type": "int"},
			{"key": "reward_xp", "type": "int"}, {"key": "reward_gold", "type": "int"},
		],
	},
	"Диалоги": {
		"file": "dialogues.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"},
			{"key": "lines", "type": "json"},
			{"key": "choices", "type": "json"},
		],
	},
	"Пресет": {
		"file": "preset.json", "root": [], "single": true,
		"fields": [
			{"key": "id", "type": "str"}, {"key": "name_ru", "type": "str"},
			{"key": "desc_ru", "type": "text"}, {"key": "author", "type": "str"},
			{"key": "version", "type": "int"},
		],
	},
	"Карта: блоки": {
		"file": "maps/*", "root": ["blocks"],
		"fields": [
			{"key": "shape", "type": "enum", "options": ["box", "ramp", "stairs", "cylinder"]},
			{"key": "pos", "type": "vec3"}, {"key": "size", "type": "vec3"},
			{"key": "rot", "type": "float"},
			{"key": "from", "type": "json"}, {"key": "to", "type": "json"},
			{"key": "width", "type": "float"}, {"key": "steps", "type": "int"},
			{"key": "radius", "type": "float"}, {"key": "height", "type": "float"},
			{"key": "tex", "type": "str"}, {"key": "uv", "type": "float"},
		],
	},
	"Карта: здания": {
		"file": "maps/*", "root": ["buildings"],
		"fields": [
			{"key": "pos", "type": "vec2"}, {"key": "size", "type": "vec3"},
			{"key": "tex", "type": "str"}, {"key": "sign", "type": "str"},
			{"key": "sign_side", "type": "enum", "options": ["n", "s", "e", "w"], "nullable": true},
		],
	},
	"Карта: свет": {
		"file": "maps/*", "root": ["lights"],
		"fields": [
			{"key": "pos", "type": "vec3"}, {"key": "color", "type": "color"},
			{"key": "energy", "type": "float"}, {"key": "range", "type": "float"},
		],
	},
	"Карта: пропсы": {
		"file": "maps/*", "root": ["props"],
		"fields": [
			{"key": "tex", "type": "str"}, {"key": "pos", "type": "vec3"},
			{"key": "px", "type": "float"},
		],
	},
	"Карта: флэты": {
		"file": "maps/*", "root": ["flats"],
		"fields": [
			{"key": "tex", "type": "str"}, {"key": "pos", "type": "vec3"},
			{"key": "rot", "type": "float"}, {"key": "px", "type": "float"},
			{"key": "glow", "type": "bool"},
		],
	},
	"Карта: глоу-каналы": {
		"file": "maps/*", "root": ["glows"],
		"fields": [
			{"key": "pos", "type": "vec3"}, {"key": "size", "type": "vec3"},
			{"key": "tex", "type": "str"}, {"key": "emission", "type": "color"},
			{"key": "uv", "type": "float"},
		],
	},
	"Карта: спавн врагов": {
		"file": "maps/*", "root": ["spawns", "spawn_enemies"],
		"fields": [
			{"key": "kind", "type": "dyn_enum", "source": "enemies"},
			{"key": "x", "type": "float"}, {"key": "z", "type": "float"},
		],
	},
	"Карта: спавн лута": {
		"file": "maps/*", "root": ["spawns", "spawn_items"],
		"fields": [
			{"key": "kind", "type": "dyn_enum", "source": "items"},
			{"key": "x", "type": "float"}, {"key": "z", "type": "float"},
		],
	},
	"Карта: спавн патронов": {
		"file": "maps/*", "root": ["spawns", "spawn_ammo"],
		"fields": [
			{"key": "kind", "type": "enum", "options": ["bullets", "shells", "rockets", "cells"]},
			{"key": "amount", "type": "int"},
			{"key": "x", "type": "float"}, {"key": "z", "type": "float"},
		],
	},
	"Карта: спавн оружия": {
		"file": "maps/*", "root": ["spawns", "spawn_weapons"],
		"fields": [
			{"key": "kind", "type": "dyn_enum", "source": "weapons"},
			{"key": "x", "type": "float"}, {"key": "z", "type": "float"},
		],
	},
}

## Фундаментальные файлы игры — их защищает «Замок ядра» (read-only на диске).
const CORE_FILES := [
	"res://main.tscn", "res://main_menu.tscn",
	"res://project.godot", "res://OpenHeart.gdextension",
]

# ── Состояние ─────────────────────────────────────────────────────────────────

var preset_id := "core"
var category := "Оружие"
var current_map := "hub.json"   # выбранный файл в maps/ (категории "maps/*")
var file_cache := {}       # относительный путь → распарсенные данные (общий на файл!)
var dirty := false

var preset_pick: OptionButton
var map_pick: OptionButton
var cat_list: ItemList
var rec_list: ItemList
var rec_map: Array[int] = []    # строка списка → индекс записи (фильтр поиска)
var search_edit: LineEdit
var form_box: VBoxContainer
var status: Label
var lock_btn: Button
var new_preset_edit: LineEdit

# ИИ-генерация текстур (окно поверх панели)
var gen_window: Window
var gen_type: OptionButton
var gen_id: LineEdit
var gen_desc: TextEdit
var gen_prompt_preview: Label
var gen_status: Label
var gen_preview: TextureRect
var gen_btn: Button
var gen_thread: Thread
var gen_templates := {}


func _ready() -> void:
	name = "OpenHeartEditor"
	set_anchors_preset(Control.PRESET_FULL_RECT)
	_build_ui()
	_scan_presets()
	_scan_maps()
	_load_category()


func _exit_tree() -> void:
	# Плагин выключают/перезагружают: дождаться потока генерации, иначе Godot
	# ругается на Thread без wait_to_finish, а _gen_done прилетит в мёртвый узел.
	if gen_thread != null:
		gen_thread.wait_to_finish()
		gen_thread = null


# ── UI каркас ─────────────────────────────────────────────────────────────────

func _build_ui() -> void:
	var root := VBoxContainer.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(root)

	# Верхняя панель
	var top := HBoxContainer.new()
	root.add_child(top)

	top.add_child(_mk_label("Пресет:"))
	preset_pick = OptionButton.new()
	preset_pick.item_selected.connect(_on_preset_selected)
	top.add_child(preset_pick)

	top.add_child(_mk_label("Карта:"))
	map_pick = OptionButton.new()
	map_pick.tooltip_text = "Какой файл из maps/ редактируют категории «Карта: …»"
	map_pick.item_selected.connect(_on_map_selected)
	top.add_child(map_pick)

	new_preset_edit = LineEdit.new()
	new_preset_edit.placeholder_text = "id нового пресета…"
	new_preset_edit.custom_minimum_size.x = 160
	top.add_child(new_preset_edit)
	var np := Button.new()
	np.text = "Создать копию"
	np.tooltip_text = "Скопировать текущий пресет в presets/<id> — отдельная игра"
	np.pressed.connect(_on_new_preset)
	top.add_child(np)

	top.add_spacer(false)

	var save_btn := Button.new()
	save_btn.text = "💾 Сохранить пресет"
	save_btn.pressed.connect(_save_all)
	top.add_child(save_btn)

	lock_btn = Button.new()
	lock_btn.toggle_mode = true
	lock_btn.toggled.connect(_on_lock_toggled)
	top.add_child(lock_btn)
	_refresh_lock_button()

	var gen_open := Button.new()
	gen_open.text = "🎨 ИИ-текстуры"
	gen_open.tooltip_text = "Сгенерировать спрайт/текстуру нейросетью (сервер настраивается в tools/aigen.json)"
	gen_open.pressed.connect(_open_gen_window)
	top.add_child(gen_open)

	status = _mk_label("")
	status.modulate = Color(1.0, 0.7, 0.9)
	top.add_child(status)

	# Основная область
	var split := HSplitContainer.new()
	split.size_flags_vertical = Control.SIZE_EXPAND_FILL
	root.add_child(split)

	var left := VBoxContainer.new()
	left.custom_minimum_size.x = 420
	split.add_child(left)

	left.add_child(_mk_label("Категория"))
	cat_list = ItemList.new()
	cat_list.custom_minimum_size.y = 230
	for k in SCHEMAS.keys():
		cat_list.add_item(k)
	cat_list.item_selected.connect(_on_category_selected)
	left.add_child(cat_list)

	left.add_child(_mk_label("Записи"))
	search_edit = LineEdit.new()
	search_edit.placeholder_text = "🔎 поиск по записям…"
	search_edit.clear_button_enabled = true
	search_edit.text_changed.connect(func(_t): _refresh_records(_selected_index()))
	left.add_child(search_edit)
	rec_list = ItemList.new()
	rec_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	rec_list.item_selected.connect(_on_record_selected)
	left.add_child(rec_list)

	var crud := HBoxContainer.new()
	left.add_child(crud)
	var b_add := Button.new(); b_add.text = "＋ Добавить"; b_add.pressed.connect(_on_add)
	var b_dup := Button.new(); b_dup.text = "⧉ Дублировать"; b_dup.pressed.connect(_on_dup)
	var b_del := Button.new(); b_del.text = "🗑 Удалить"; b_del.pressed.connect(_on_del)
	crud.add_child(b_add); crud.add_child(b_dup); crud.add_child(b_del)

	var scroll := ScrollContainer.new()
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	split.add_child(scroll)
	form_box = VBoxContainer.new()
	form_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(form_box)


func _mk_label(text: String) -> Label:
	var l := Label.new()
	l.text = text
	return l


func _set_status(text: String, ok := true) -> void:
	status.text = text
	status.modulate = Color(0.6, 1.0, 0.7) if ok else Color(1.0, 0.5, 0.5)


# ── Пресеты ───────────────────────────────────────────────────────────────────

func _preset_root() -> String:
	# как content.rs::preset_base — сначала встроенные, затем user:// (моды)
	var res := "res://presets/%s" % preset_id
	if FileAccess.file_exists("%s/preset.json" % res):
		return res
	var user := "user://presets/%s" % preset_id
	if FileAccess.file_exists("%s/preset.json" % user):
		return user
	return res


func _scan_presets() -> void:
	preset_pick.clear()
	var found: Array[String] = []
	for root in ["res://presets", "user://presets"]:
		var dir := DirAccess.open(root)
		if dir:
			for d in dir.get_directories():
				if not d.begins_with(".") and not found.has(d):
					found.append(d)
	found.sort()
	for i in found.size():
		preset_pick.add_item(found[i])
		if found[i] == preset_id:
			preset_pick.select(i)
	if not found.has(preset_id) and found.size() > 0:
		preset_id = found[0]
		preset_pick.select(0)


## Файлы карт текущего пресета (maps/*.json) → выпадающий список.
func _scan_maps() -> void:
	map_pick.clear()
	var found: Array[String] = []
	var dir := DirAccess.open("%s/maps" % _preset_root())
	if dir:
		for f in dir.get_files():
			if f.ends_with(".json"):
				found.append(f)
	found.sort()
	for i in found.size():
		map_pick.add_item(found[i])
		if found[i] == current_map:
			map_pick.select(i)
	if not found.has(current_map):
		current_map = found[0] if found.size() > 0 else "hub.json"
		if found.size() > 0:
			map_pick.select(0)


func _on_map_selected(idx: int) -> void:
	current_map = map_pick.get_item_text(idx)
	if category.begins_with("Карта"):
		_refresh_records(0)
	_set_status("Карта: %s" % current_map)


func _on_preset_selected(idx: int) -> void:
	if dirty:
		_save_all()
	preset_id = preset_pick.get_item_text(idx)
	file_cache.clear()
	_scan_maps()
	_load_category()
	_set_status("Пресет: %s" % preset_id)


func _on_new_preset() -> void:
	var new_id := new_preset_edit.text.strip_edges()
	if new_id.is_empty() or not new_id.is_valid_filename():
		_set_status("Некорректный id пресета", false)
		return
	var src := _preset_root()
	var dst := "res://presets/%s" % new_id
	if DirAccess.dir_exists_absolute(dst):
		_set_status("Пресет %s уже существует" % new_id, false)
		return
	_copy_dir(src, dst)
	# правим манифест
	var mp := "%s/preset.json" % dst
	var info = _read_json(mp)
	if typeof(info) == TYPE_DICTIONARY:
		info["id"] = new_id
		info["name_ru"] = new_id
		_write_json(mp, info)
	preset_id = new_id
	file_cache.clear()
	_scan_presets()
	_load_category()
	_set_status("Создан пресет %s — теперь это отдельная игра" % new_id)


func _copy_dir(src: String, dst: String) -> void:
	DirAccess.make_dir_recursive_absolute(dst)
	var dir := DirAccess.open(src)
	if not dir:
		return
	for f in dir.get_files():
		dir.copy("%s/%s" % [src, f], "%s/%s" % [dst, f])
	for d in dir.get_directories():
		_copy_dir("%s/%s" % [src, d], "%s/%s" % [dst, d])


# ── Файлы и данные ────────────────────────────────────────────────────────────

func _read_json(path: String):
	var f := FileAccess.open(path, FileAccess.READ)
	if not f:
		return null
	return JSON.parse_string(f.get_as_text())


func _write_json(path: String, data) -> bool:
	var f := FileAccess.open(path, FileAccess.WRITE)
	if not f:
		return false
	f.store_string(JSON.stringify(data, "  ", false))
	return true


func _schema() -> Dictionary:
	return SCHEMAS[category]


## Файл схемы с подстановкой выбранной карты ("maps/*" → maps/<current_map>).
func _schema_file(schema: Dictionary = {}) -> String:
	var f: String = (schema if not schema.is_empty() else _schema())["file"]
	return "maps/%s" % current_map if f == "maps/*" else f


## Содержимое файла пресета через общий кэш (несохранённые правки видны всем).
func _cached_file(rel: String, default_root):
	if not file_cache.has(rel):
		var parsed = _read_json("%s/%s" % [_preset_root(), rel])
		if parsed == null:
			parsed = default_root
		file_cache[rel] = parsed
	return file_cache[rel]


## Массив записей текущей категории (внутри общего файла — по root-пути).
## Для single-схем (preset.json) возвращает [сам словарь].
func _records() -> Array:
	var schema := _schema()
	var rel := _schema_file()
	var is_single: bool = schema.get("single", false)
	var node = _cached_file(rel, {} if (is_single or not (schema["root"] as Array).is_empty()) else [])
	if is_single:
		return [node] if typeof(node) == TYPE_DICTIONARY else []
	var root: Array = schema["root"]
	for i in root.size():
		if typeof(node) != TYPE_DICTIONARY:
			return []
		# недостающую секцию создаём в кэше — иначе «Добавить» пишет в пустоту
		if not node.has(root[i]):
			node[root[i]] = [] if i == root.size() - 1 else {}
		node = node[root[i]]
	return node if typeof(node) == TYPE_ARRAY else []


# ── Списки и форма ────────────────────────────────────────────────────────────

func _on_category_selected(idx: int) -> void:
	category = cat_list.get_item_text(idx)
	_load_category()


func _load_category() -> void:
	for i in cat_list.item_count:
		if cat_list.get_item_text(i) == category:
			cat_list.select(i)
			break
	_refresh_records(0)


## select_idx — индекс ЗАПИСИ (не строки списка); учитывает фильтр поиска.
func _refresh_records(select_idx: int) -> void:
	rec_list.clear()
	rec_map.clear()
	var recs := _records()
	var filter := search_edit.text.strip_edges().to_lower() if search_edit else ""
	for i in recs.size():
		var title := _record_title(recs[i])
		if filter.is_empty() or filter in title.to_lower():
			rec_list.add_item(title)
			rec_map.append(i)
	if rec_map.is_empty():
		_clear_form()
		return
	select_idx = clampi(select_idx, 0, recs.size() - 1)
	var row := rec_map.find(select_idx)
	if row < 0:
		row = 0
	rec_list.select(row)
	_build_form(rec_map[row])


func _record_title(r) -> String:
	if typeof(r) != TYPE_DICTIONARY:
		return str(r)
	var id_part = r.get("id", r.get("kind", r.get("shape", r.get("tex", "запись"))))
	var name_part = r.get("name_ru", r.get("name", r.get("title_ru", "")))
	return "%s — %s" % [id_part, name_part] if name_part else str(id_part)


## Индекс выбранной ЗАПИСИ (через rec_map, т.к. список может быть отфильтрован).
func _selected_index() -> int:
	var sel := rec_list.get_selected_items()
	if sel.size() == 0 or sel[0] >= rec_map.size():
		return -1
	return rec_map[sel[0]]


func _on_record_selected(row: int) -> void:
	if row >= 0 and row < rec_map.size():
		_build_form(rec_map[row])


func _clear_form() -> void:
	for c in form_box.get_children():
		c.queue_free()


func _build_form(idx: int) -> void:
	_clear_form()
	var recs := _records()
	if idx < 0 or idx >= recs.size():
		return
	var rec: Dictionary = recs[idx]

	for field in _schema()["fields"]:
		var key: String = field["key"]
		var row := HBoxContainer.new()
		var lbl := _mk_label(key)
		lbl.custom_minimum_size.x = 150
		row.add_child(lbl)
		var editor := _make_field_editor(field, rec, idx)
		editor.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(editor)
		form_box.add_child(row)


## Редактор одного поля; пишет прямо в запись при изменении.
func _make_field_editor(field: Dictionary, rec: Dictionary, rec_idx: int) -> Control:
	var key: String = field["key"]
	var t: String = field["type"]
	var val = rec.get(key)

	match t:
		"str":
			var e := LineEdit.new()
			e.text = str(val) if val != null else ""
			e.text_changed.connect(func(txt): rec[key] = txt; _mark_dirty(rec_idx))
			return e
		"text":
			var e := TextEdit.new()
			e.custom_minimum_size.y = 64
			e.text = str(val) if val != null else ""
			e.text_changed.connect(func(): rec[key] = e.text; _mark_dirty(rec_idx))
			return e
		"float":
			var e := SpinBox.new()
			e.step = 0.05
			e.min_value = -100000.0
			e.max_value = 100000.0
			e.value = float(val) if val != null else 0.0
			e.value_changed.connect(func(v): rec[key] = v; _mark_dirty(rec_idx))
			return e
		"int":
			var e := SpinBox.new()
			e.step = 1
			e.min_value = -1000000
			e.max_value = 1000000
			e.value = int(val) if val != null else 0
			e.value_changed.connect(func(v): rec[key] = int(v); _mark_dirty(rec_idx))
			return e
		"bool":
			var e := CheckBox.new()
			e.button_pressed = bool(val) if val != null else false
			e.toggled.connect(func(on): rec[key] = on; _mark_dirty(rec_idx))
			return e
		"vec2", "vec3":
			var dims := 2 if t == "vec2" else 3
			var box := HBoxContainer.new()
			var arr: Array = val if typeof(val) == TYPE_ARRAY and (val as Array).size() == dims else []
			var spins: Array = []
			for d in dims:
				var sp := SpinBox.new()
				sp.step = 0.1
				sp.min_value = -100000.0
				sp.max_value = 100000.0
				sp.value = float(arr[d]) if arr.size() == dims else 0.0
				sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
				spins.append(sp)
				box.add_child(sp)
			for sp in spins:
				(sp as SpinBox).value_changed.connect(func(_v):
					var out := []
					for s in spins:
						out.append((s as SpinBox).value)
					rec[key] = out
					_mark_dirty(rec_idx)
				)
			return box
		"color":
			# [r, g, b] (0..1) через палитру
			var e := ColorPickerButton.new()
			e.edit_alpha = false
			e.custom_minimum_size = Vector2(110, 0)
			if typeof(val) == TYPE_ARRAY and (val as Array).size() >= 3:
				e.color = Color(float(val[0]), float(val[1]), float(val[2]))
			e.color_changed.connect(func(col: Color):
				rec[key] = [col.r, col.g, col.b]
				_mark_dirty(rec_idx)
			)
			return e
		"enum", "dyn_enum":
			var e := OptionButton.new()
			var values: Array = []
			if field.get("nullable", false):
				values.append("")
			if t == "enum":
				values.append_array(field["options"])
			else:
				values.append_array(_dyn_options(str(field.get("source", ""))))
			var cur_s := str(val) if val != null else ""
			if not cur_s.is_empty() and not values.has(cur_s):
				values.append(cur_s)   # не терять значение, которого нет в источнике
			for v in values:
				e.add_item("(нет)" if str(v).is_empty() else str(v))
			var cur := values.find(cur_s)
			if cur >= 0:
				e.select(cur)
			e.item_selected.connect(func(i):
				rec[key] = null if str(values[i]).is_empty() else values[i]
				_mark_dirty(rec_idx)
			)
			return e
		_:
			# json: вложенные структуры (массивы, объекты, null)
			var e := TextEdit.new()
			e.custom_minimum_size.y = 56
			e.text = JSON.stringify(val) if val != null else "null"
			e.text_changed.connect(func():
				var parsed = JSON.parse_string(e.text)
				if parsed == null and e.text.strip_edges() != "null":
					_set_status("%s: некорректный JSON" % key, false)
				else:
					rec[key] = parsed
					_mark_dirty(rec_idx)
					_set_status("")
			)
			return e


func _mark_dirty(rec_idx: int) -> void:
	dirty = true
	var row := rec_map.find(rec_idx)
	if row >= 0 and row < rec_list.item_count:
		rec_list.set_item_text(row, _record_title(_records()[rec_idx]))


# ── Динамические списки значений (из данных пресета) ──────────────────────────

## id всех записей массива (сам массив или словарь с ключом root_key).
func _ids_of(data, root_key: String) -> Array:
	var arr = data
	if not root_key.is_empty() and typeof(data) == TYPE_DICTIONARY:
		arr = data.get(root_key, [])
	var out: Array = []
	if typeof(arr) == TYPE_ARRAY:
		for r in arr:
			if typeof(r) == TYPE_DICTIONARY and r.has("id"):
				out.append(str(r["id"]))
	return out


## Варианты для dyn_enum-полей. Читает через file_cache — несохранённые
## правки соседних категорий сразу видны в выпадающих списках.
func _dyn_options(source: String) -> Array:
	match source:
		"npcs":
			return _ids_of(_cached_file("npcs.json", []), "")
		"enemies":
			return _ids_of(_cached_file("enemies.json", {}), "enemies")
		"items":
			return _ids_of(_cached_file("items.json", {}), "items")
		"targets":   # цели квестов: враги (kill) + предметы (collect)
			var out := _dyn_options("enemies")
			out.append_array(_dyn_options("items"))
			out.append("heart_1up")
			return out
		"quests":
			return _ids_of(_cached_file("quests.json", []), "")
		"weapons":
			return _ids_of(_cached_file("weapons.json", []), "")
		"scenes":    # "story" (динамика story.rs) + сцены dialogues.json
			var out: Array = ["story"]
			out.append_array(_ids_of(_cached_file("dialogues.json", []), ""))
			return out
		"enemy_sprites":   # скан листов enemy_*.png — новые спрайты сразу в списке
			var out: Array = []
			var dir := DirAccess.open("res://assets/sprites/characters")
			if dir:
				for f in dir.get_files():
					if f.begins_with("enemy_") and f.ends_with(".png"):
						out.append(f.trim_prefix("enemy_").trim_suffix(".png"))
			out.sort()
			return out
	return []


# ── CRUD ─────────────────────────────────────────────────────────────────────

func _single_guard() -> bool:
	if _schema().get("single", false):
		_set_status("«%s» — одиночная запись, CRUD не применим" % category, false)
		return true
	return false


func _on_add() -> void:
	if _single_guard():
		return
	var recs := _records()
	var blank := {}
	for field in _schema()["fields"]:
		match field["type"]:
			"str", "text": blank[field["key"]] = ""
			"float": blank[field["key"]] = 0.0
			"int": blank[field["key"]] = 0
			"bool": blank[field["key"]] = false
			"enum": blank[field["key"]] = null if field.get("nullable", false) else field["options"][0]
			"dyn_enum":
				var opts := _dyn_options(str(field.get("source", "")))
				blank[field["key"]] = null if field.get("nullable", false) \
					else (opts[0] if opts.size() > 0 else "")
			"vec2": blank[field["key"]] = [0.0, 0.0]
			"vec3": blank[field["key"]] = [0.0, 0.0, 0.0]
			"color": blank[field["key"]] = [1.0, 1.0, 1.0]
			_: blank[field["key"]] = null
	if blank.has("id"):
		blank["id"] = "new_%d" % (recs.size() + 1)
	recs.append(blank)
	dirty = true
	search_edit.text = ""   # новая запись не должна прятаться за фильтром
	_refresh_records(recs.size() - 1)


func _on_dup() -> void:
	if _single_guard():
		return
	var idx := _selected_index()
	var recs := _records()
	if idx < 0 or idx >= recs.size():
		return
	var copy = recs[idx].duplicate(true)
	if typeof(copy) == TYPE_DICTIONARY and copy.has("id"):
		copy["id"] = str(copy["id"]) + "_copy"
	recs.insert(idx + 1, copy)
	dirty = true
	_refresh_records(idx + 1)


func _on_del() -> void:
	if _single_guard():
		return
	var idx := _selected_index()
	var recs := _records()
	if idx < 0 or idx >= recs.size():
		return
	recs.remove_at(idx)
	dirty = true
	_refresh_records(idx)


# ── Сохранение ────────────────────────────────────────────────────────────────

func _save_all() -> void:
	var saved := 0
	for rel in file_cache.keys():
		if _write_json("%s/%s" % [_preset_root(), rel], file_cache[rel]):
			saved += 1
	dirty = false
	var warnings := _validate_preset()
	if warnings.is_empty():
		_set_status("Сохранено файлов: %d (пресет %s)" % [saved, preset_id])
	else:
		for w in warnings:
			print("[oh_editor] ⚠ %s" % w)
		_set_status("Сохранено: %d; ⚠ битых ссылок: %d — список в Output. Первая: %s"
			% [saved, warnings.size(), warnings[0]], false)
	# обновить FileSystem-докcy редактора
	EditorInterface.get_resource_filesystem().scan()


## Проверка ссылочной целостности пресета (не блокирует сохранение —
## предупреждает; зеркалит cargo-тест content.rs::preset_tests).
func _validate_preset() -> Array:
	var warnings: Array = []
	var npc_ids   := _dyn_options("npcs")
	var enemy_ids := _dyn_options("enemies")
	var item_ids  := _dyn_options("items")
	var quest_ids := _dyn_options("quests")
	var scene_ids := _ids_of(_cached_file("dialogues.json", []), "")

	var quests = _cached_file("quests.json", [])
	if typeof(quests) == TYPE_ARRAY:
		for q in quests:
			if typeof(q) != TYPE_DICTIONARY:
				continue
			var qid := str(q.get("id", "?"))
			if not npc_ids.has(str(q.get("giver", ""))):
				warnings.append("квест '%s': гивер '%s' не найден в NPC" % [qid, q.get("giver", "")])
			var target := str(q.get("target", ""))
			match str(q.get("kind", "")):
				"kill":
					if not enemy_ids.has(target):
						warnings.append("квест '%s': цель kill '%s' не найдена во врагах" % [qid, target])
				"collect":
					if not (item_ids.has(target) or target == "heart_1up"):
						warnings.append("квест '%s': цель collect '%s' не найдена в предметах" % [qid, target])

	var npcs = _cached_file("npcs.json", [])
	if typeof(npcs) == TYPE_ARRAY:
		for n in npcs:
			if typeof(n) != TYPE_DICTIONARY:
				continue
			var nq = n.get("quest")
			if nq != null and not str(nq).is_empty() and not quest_ids.has(str(nq)):
				warnings.append("NPC '%s': квест '%s' не найден" % [n.get("id", "?"), nq])
			var ns = n.get("scene")
			if ns != null and not str(ns).is_empty() and str(ns) != "story" \
					and not scene_ids.has(str(ns)):
				warnings.append("NPC '%s': сцены '%s' нет в dialogues.json (ок, если это встроенная story-сцена)"
					% [n.get("id", "?"), ns])

	for rel in file_cache.keys():
		if not str(rel).begins_with("maps/"):
			continue
		var m = file_cache[rel]
		if typeof(m) != TYPE_DICTIONARY:
			continue
		var spawns = m.get("spawns", {})
		if typeof(spawns) != TYPE_DICTIONARY:
			continue
		for s in spawns.get("spawn_enemies", []):
			if typeof(s) == TYPE_DICTIONARY and not enemy_ids.has(str(s.get("kind", ""))):
				warnings.append("%s: спавн врага '%s' — нет во врагах" % [rel, s.get("kind", "")])
		for s in spawns.get("spawn_items", []):
			if typeof(s) == TYPE_DICTIONARY:
				var k := str(s.get("kind", ""))
				if not (item_ids.has(k) or k == "heart_1up"):
					warnings.append("%s: спавн предмета '%s' — нет в предметах" % [rel, k])

	var scenes = _cached_file("dialogues.json", [])
	if typeof(scenes) == TYPE_ARRAY:
		for sc in scenes:
			if typeof(sc) != TYPE_DICTIONARY:
				continue
			for c in sc.get("choices", []):
				if typeof(c) != TYPE_DICTIONARY:
					continue
				var nx = c.get("next")
				if nx != null and not str(nx).is_empty() and not scene_ids.has(str(nx)):
					warnings.append("диалог '%s': next '%s' нет в dialogues.json (ок, если story-сцена)"
						% [sc.get("id", "?"), nx])
	return warnings


# ── ИИ-генерация текстур ──────────────────────────────────────────────────────
# Окно «в несколько кликов»: тип ассета + id + описание → tools/aigen.py
# (HTTP к серверу нейросети + постобработка process_sprites.py). Итоговый PNG
# ложится сразу в godot/assets/*, игра подхватывает его при следующем F5.

func _tools_dir() -> String:
	return ProjectSettings.globalize_path("res://").path_join("../tools").simplify_path()


func _open_gen_window() -> void:
	if gen_window == null:
		_build_gen_window()
	_refresh_gen_template()
	gen_window.popup_centered(Vector2i(760, 700))


func _build_gen_window() -> void:
	var tpl = _read_json(_tools_dir().path_join("aigen_templates.json"))
	gen_templates = tpl if typeof(tpl) == TYPE_DICTIONARY else {}

	gen_window = Window.new()
	gen_window.title = "ИИ-генерация текстур"
	gen_window.close_requested.connect(func(): gen_window.hide())
	add_child(gen_window)

	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	for side in ["margin_left", "margin_top", "margin_right", "margin_bottom"]:
		margin.add_theme_constant_override(side, 12)
	gen_window.add_child(margin)

	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 8)
	margin.add_child(v)

	var row_type := HBoxContainer.new()
	row_type.add_child(_mk_label("Тип ассета:"))
	gen_type = OptionButton.new()
	for k in gen_templates.keys():
		if str(k).begins_with("_"):
			continue
		var idx := gen_type.item_count
		gen_type.add_item("%s — %s" % [k, gen_templates[k].get("title_ru", "")])
		gen_type.set_item_metadata(idx, k)
	gen_type.item_selected.connect(func(_i): _refresh_gen_template())
	row_type.add_child(gen_type)
	v.add_child(row_type)

	var row_id := HBoxContainer.new()
	row_id.add_child(_mk_label("id:"))
	gen_id = LineEdit.new()
	gen_id.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row_id.add_child(gen_id)
	v.add_child(row_id)

	v.add_child(_mk_label("Описание (вставляется в шаблон промпта):"))
	gen_desc = TextEdit.new()
	gen_desc.custom_minimum_size.y = 72
	gen_desc.text_changed.connect(_refresh_gen_template)
	v.add_child(gen_desc)

	v.add_child(_mk_label("Итоговый промпт:"))
	gen_prompt_preview = Label.new()
	gen_prompt_preview.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	gen_prompt_preview.modulate = Color(0.7, 0.7, 0.8)
	v.add_child(gen_prompt_preview)

	gen_btn = Button.new()
	gen_btn.text = "⚡ Сгенерировать"
	gen_btn.pressed.connect(_on_generate)
	v.add_child(gen_btn)

	gen_status = _mk_label("")
	gen_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	v.add_child(gen_status)

	gen_preview = TextureRect.new()
	gen_preview.custom_minimum_size = Vector2(280, 280)
	gen_preview.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	gen_preview.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	gen_preview.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	gen_preview.size_flags_vertical = Control.SIZE_EXPAND_FILL
	v.add_child(gen_preview)


func _gen_selected_type() -> String:
	if gen_type.selected < 0:
		return ""
	return str(gen_type.get_item_metadata(gen_type.selected))


func _refresh_gen_template() -> void:
	var t := _gen_selected_type()
	var tpl: Dictionary = gen_templates.get(t, {})
	gen_id.placeholder_text = str(tpl.get("id_hint", "id"))
	var desc := gen_desc.text.strip_edges()
	gen_prompt_preview.text = str(tpl.get("prompt", "{desc}")) \
		.format({"desc": desc if not desc.is_empty() else "<описание>"})


func _on_generate() -> void:
	var type := _gen_selected_type()
	var id := gen_id.text.strip_edges()
	var desc := gen_desc.text.strip_edges().replace("\n", " ")
	if type.is_empty() or id.is_empty() or desc.is_empty():
		gen_status.text = "Заполни тип, id и описание"
		return
	if gen_thread != null:
		gen_status.text = "Генерация уже идёт…"
		return

	var py := "python"
	var cfg = _read_json(_tools_dir().path_join("aigen.json"))
	if typeof(cfg) == TYPE_DICTIONARY:
		py = str(cfg.get("python", "python"))

	gen_btn.disabled = true
	gen_status.text = "Генерация… (%s @ %s)" % [
		cfg.get("backend", "?") if typeof(cfg) == TYPE_DICTIONARY else "?",
		cfg.get("url", "?") if typeof(cfg) == TYPE_DICTIONARY else "?"]
	var args := PackedStringArray([_tools_dir().path_join("aigen.py"), type, id, desc])
	gen_thread = Thread.new()
	gen_thread.start(_gen_worker.bind(py, args))


func _gen_worker(py: String, args: PackedStringArray) -> void:
	var out := []
	var code := OS.execute(py, args, out, true)
	call_deferred("_gen_done", code, out)


func _gen_done(code: int, out: Array) -> void:
	if gen_thread != null:
		gen_thread.wait_to_finish()
		gen_thread = null
	if not is_instance_valid(gen_btn):
		return  # окно уже уничтожено (плагин выключили во время генерации)
	gen_btn.disabled = false

	var text := ""
	for o in out:
		text += str(o)
	var ok_path := ""
	var err_msg := ""
	for line in text.split("\n"):
		var l: String = line.strip_edges()
		if l.begins_with("OK "):
			ok_path = l.substr(3)
		elif l.begins_with("ERR "):
			err_msg = l.substr(4)

	if code == 0 and not ok_path.is_empty():
		gen_status.text = "Готово: %s" % ok_path
		var img := Image.load_from_file(ok_path)
		if img != null:
			gen_preview.texture = ImageTexture.create_from_image(img)
		EditorInterface.get_resource_filesystem().scan()
	elif not err_msg.is_empty():
		gen_status.text = "Ошибка: %s" % err_msg
	else:
		gen_status.text = "Сбой генерации (код %d). Вывод:\n%s" % [code, text.right(600)]


# ── Замок ядра ────────────────────────────────────────────────────────────────
# Контент-мейкер работает только с data-файлами. Кнопка ставит/снимает read-only
# на фундаментальных файлах игры (сцены, project.godot, .gdextension), чтобы их
# нельзя было случайно перезаписать даже из самого Godot-редактора.

func _core_locked() -> bool:
	var probe := ProjectSettings.globalize_path(CORE_FILES[0])
	var out := []
	OS.execute("attrib", [probe.replace("/", "\\")], out)
	return out.size() > 0 and "R" in str(out[0]).split(probe.replace("/", "\\"))[0]


func _on_lock_toggled(on: bool) -> void:
	var flag := "+R" if on else "-R"
	for f in CORE_FILES:
		var p := ProjectSettings.globalize_path(f).replace("/", "\\")
		OS.execute("attrib", [flag, p])
	_refresh_lock_button()
	_set_status("Ядро игры защищено (read-only)" if on else "Защита ядра снята")


func _refresh_lock_button() -> void:
	var locked := _core_locked()
	lock_btn.set_pressed_no_signal(locked)
	lock_btn.text = "🔒 Ядро защищено" if locked else "🔓 Защитить ядро"
	lock_btn.tooltip_text = "Ставит read-only на main.tscn, main_menu.tscn, project.godot и .gdextension.\nКонтент-мейкер меняет игру только через эту панель."
