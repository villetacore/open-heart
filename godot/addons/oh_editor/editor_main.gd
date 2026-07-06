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
# json — вложенные структуры (массивы/объекты) как текст c проверкой синтаксиса.

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
			{"key": "sprite", "type": "enum",
			 "options": ["grunt", "fast", "heavy", "brute", "sniper", "cultist"]},
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
			{"key": "sprite", "type": "str"}, {"key": "pos", "type": "json"},
			{"key": "color", "type": "json"}, {"key": "scene", "type": "str"},
			{"key": "quest", "type": "str"},
		],
	},
	"Квесты": {
		"file": "quests.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "title_ru", "type": "str"},
			{"key": "desc_ru", "type": "text"}, {"key": "giver", "type": "str"},
			{"key": "kind", "type": "enum", "options": ["kill", "collect", "clear_dungeon"]},
			{"key": "target", "type": "str"}, {"key": "count", "type": "int"},
			{"key": "reward_xp", "type": "int"}, {"key": "reward_gold", "type": "int"},
		],
	},
	"Карта: блоки": {
		"file": "maps/hub.json", "root": ["blocks"],
		"fields": [
			{"key": "shape", "type": "enum", "options": ["box", "ramp", "stairs", "cylinder"]},
			{"key": "pos", "type": "json"}, {"key": "size", "type": "json"},
			{"key": "rot", "type": "float"},
			{"key": "from", "type": "json"}, {"key": "to", "type": "json"},
			{"key": "width", "type": "float"}, {"key": "steps", "type": "int"},
			{"key": "radius", "type": "float"}, {"key": "height", "type": "float"},
			{"key": "tex", "type": "str"}, {"key": "uv", "type": "float"},
		],
	},
	"Карта: здания": {
		"file": "maps/hub.json", "root": ["buildings"],
		"fields": [
			{"key": "pos", "type": "json"}, {"key": "size", "type": "json"},
			{"key": "tex", "type": "str"}, {"key": "sign", "type": "str"},
			{"key": "sign_side", "type": "enum", "options": ["n", "s", "e", "w"]},
		],
	},
	"Карта: свет": {
		"file": "maps/hub.json", "root": ["lights"],
		"fields": [
			{"key": "pos", "type": "json"}, {"key": "color", "type": "json"},
			{"key": "energy", "type": "float"}, {"key": "range", "type": "float"},
		],
	},
	"Карта: пропсы": {
		"file": "maps/hub.json", "root": ["props"],
		"fields": [
			{"key": "tex", "type": "str"}, {"key": "pos", "type": "json"},
			{"key": "px", "type": "float"},
		],
	},
	"Карта: спавн врагов": {
		"file": "maps/hub.json", "root": ["spawns", "spawn_enemies"],
		"fields": [
			{"key": "kind", "type": "str"}, {"key": "x", "type": "float"},
			{"key": "z", "type": "float"},
		],
	},
	"Карта: спавн лута": {
		"file": "maps/hub.json", "root": ["spawns", "spawn_items"],
		"fields": [
			{"key": "kind", "type": "str"}, {"key": "x", "type": "float"},
			{"key": "z", "type": "float"},
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
var file_cache := {}       # относительный путь → распарсенные данные (общий на файл!)
var dirty := false

var preset_pick: OptionButton
var cat_list: ItemList
var rec_list: ItemList
var form_box: VBoxContainer
var status: Label
var lock_btn: Button
var new_preset_edit: LineEdit


func _ready() -> void:
	name = "OpenHeartEditor"
	set_anchors_preset(Control.PRESET_FULL_RECT)
	_build_ui()
	_scan_presets()
	_load_category()


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
	return "res://presets/%s" % preset_id


func _scan_presets() -> void:
	preset_pick.clear()
	var found: Array[String] = []
	var dir := DirAccess.open("res://presets")
	if dir:
		for d in dir.get_directories():
			if not d.begins_with("."):
				found.append(d)
	found.sort()
	for i in found.size():
		preset_pick.add_item(found[i])
		if found[i] == preset_id:
			preset_pick.select(i)
	if not found.has(preset_id) and found.size() > 0:
		preset_id = found[0]
		preset_pick.select(0)


func _on_preset_selected(idx: int) -> void:
	if dirty:
		_save_all()
	preset_id = preset_pick.get_item_text(idx)
	file_cache.clear()
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


## Массив записей текущей категории (внутри общего файла — по root-пути).
func _records() -> Array:
	var rel: String = _schema()["file"]
	if not file_cache.has(rel):
		var parsed = _read_json("%s/%s" % [_preset_root(), rel])
		if parsed == null:
			parsed = [] if (_schema()["root"] as Array).is_empty() else {}
		file_cache[rel] = parsed
	var node = file_cache[rel]
	for key in _schema()["root"]:
		if typeof(node) == TYPE_DICTIONARY and node.has(key):
			node = node[key]
		else:
			return []
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


func _refresh_records(select_idx: int) -> void:
	rec_list.clear()
	var recs := _records()
	for r in recs:
		rec_list.add_item(_record_title(r))
	if recs.size() > 0:
		select_idx = clampi(select_idx, 0, recs.size() - 1)
		rec_list.select(select_idx)
		_build_form(select_idx)
	else:
		_clear_form()


func _record_title(r) -> String:
	if typeof(r) != TYPE_DICTIONARY:
		return str(r)
	var id_part = r.get("id", r.get("kind", r.get("shape", r.get("tex", "запись"))))
	var name_part = r.get("name_ru", r.get("name", r.get("title_ru", "")))
	return "%s — %s" % [id_part, name_part] if name_part else str(id_part)


func _selected_index() -> int:
	var sel := rec_list.get_selected_items()
	return sel[0] if sel.size() > 0 else -1


func _on_record_selected(idx: int) -> void:
	_build_form(idx)


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
		"enum":
			var e := OptionButton.new()
			var opts: Array = field["options"]
			for o in opts:
				e.add_item(o)
			var cur := opts.find(val)
			if cur >= 0:
				e.select(cur)
			e.item_selected.connect(func(i): rec[key] = opts[i]; _mark_dirty(rec_idx))
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
	if rec_idx >= 0 and rec_idx < rec_list.item_count:
		rec_list.set_item_text(rec_idx, _record_title(_records()[rec_idx]))


# ── CRUD ─────────────────────────────────────────────────────────────────────

func _on_add() -> void:
	var recs := _records()
	var blank := {}
	for field in _schema()["fields"]:
		match field["type"]:
			"str", "text": blank[field["key"]] = ""
			"float": blank[field["key"]] = 0.0
			"int": blank[field["key"]] = 0
			"bool": blank[field["key"]] = false
			"enum": blank[field["key"]] = field["options"][0]
			_: blank[field["key"]] = null
	if blank.has("id"):
		blank["id"] = "new_%d" % (recs.size() + 1)
	recs.append(blank)
	dirty = true
	_refresh_records(recs.size() - 1)


func _on_dup() -> void:
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
	_set_status("Сохранено файлов: %d (пресет %s)" % [saved, preset_id])
	# обновить FileSystem-докcy редактора
	EditorInterface.get_resource_filesystem().scan()


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
