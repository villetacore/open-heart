@tool
extends Control
## OpenHeart Editor — данные, карта, текстуры, анимации.

# ─────────────────────────────── СХЕМЫ ────────────────────────────────────────

const SCHEMAS := {
	"Оружие": {
		"file": "weapons.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"}, {"key": "name_ru", "type": "str"},
			{"key": "damage", "type": "float"},
			{"key": "dmg_type", "type": "enum", "options": ["physical","fire","energy","void"]},
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
			{"key": "branch", "type": "enum", "options": ["survival","offense","utility"]},
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
			{"key": "behavior", "type": "enum", "options": ["melee", "ranged"], "nullable": true},
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
			{"key": "category", "type": "enum", "options": ["consumable","currency","key"]},
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
			{"key": "kind", "type": "enum", "options": ["kill","collect","clear_dungeon"]},
			{"key": "target", "type": "dyn_enum", "source": "targets"},
			{"key": "count", "type": "int"},
			{"key": "reward_xp", "type": "int"}, {"key": "reward_gold", "type": "int"},
		],
	},
	"Диалоги": {
		"file": "dialogues.json", "root": [],
		"fields": [
			{"key": "id", "type": "str"},
			{"key": "lines", "type": "json", "default": []},
			{"key": "choices", "type": "json", "default": []},
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
	"Данж: темы": {
		"file": "dungeon.json", "root": ["themes"],
		"fields": [
			{"key": "name_ru", "type": "str"},
			{"key": "wall", "type": "str"}, {"key": "accent", "type": "str"},
			{"key": "floor", "type": "str"}, {"key": "ceil", "type": "str"},
			{"key": "lava", "type": "str"},
			{"key": "light", "type": "color"},
		],
	},
	"Данж: пулы врагов": {
		"file": "dungeon.json", "root": ["pools"],
		"fields": [
			{"key": "min_depth", "type": "int"},
			{"key": "enemies", "type": "json", "default": []},
		],
	},
	"Данж: настройки": {
		"file": "dungeon.json", "root": ["settings"], "single": true,
		"fields": [
			{"key": "boss", "type": "dyn_enum", "source": "enemies"},
			{"key": "boss_mult", "type": "float"},
			{"key": "boss_guards", "type": "json", "default": []},
			{"key": "boss_items", "type": "json", "default": []},
			{"key": "mult_per_depth", "type": "float"},
			{"key": "weapon_cache", "type": "json", "default": []},
		],
	},
	"Лут: в комнатах": {
		"file": "loot.json", "root": ["room_items"],
		"fields": [
			{"key": "id", "type": "dyn_enum", "source": "loot_items"},
			{"key": "chance", "type": "float"},
		],
	},
	"Лут: дроп с врагов": {
		"file": "loot.json", "root": ["kill_drops"],
		"fields": [
			{"key": "kind", "type": "enum", "options": ["ammo", "item"]},
			{"key": "id", "type": "dyn_enum", "source": "loot_items", "nullable": true},
			{"key": "chance", "type": "float"},
		],
	},
	"Лут: настройки": {
		"file": "loot.json", "root": ["settings"], "single": true,
		"fields": [
			{"key": "room_ammo_chances", "type": "json", "default": []},
		],
	},
	"Карта: блоки": {
		"file": "maps/*", "root": ["blocks"],
		"fields": [
			{"key": "shape", "type": "enum", "options": ["box","ramp","stairs","cylinder"]},
			{"key": "pos", "type": "vec3"}, {"key": "size", "type": "vec3"},
			{"key": "rot", "type": "float"}, {"key": "from", "type": "json"},
			{"key": "to", "type": "json"}, {"key": "width", "type": "float"},
			{"key": "steps", "type": "int"},
			{"key": "radius", "type": "float"}, {"key": "height", "type": "float"},
			{"key": "tex", "type": "str"}, {"key": "uv", "type": "float"},
		],
	},
	"Карта: здания": {
		"file": "maps/*", "root": ["buildings"],
		"fields": [
			{"key": "pos", "type": "vec2"}, {"key": "size", "type": "vec3"},
			{"key": "tex", "type": "str"}, {"key": "sign", "type": "str"},
			{"key": "sign_side", "type": "enum", "options": ["n","s","e","w"], "nullable": true},
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
			{"key": "kind", "type": "enum", "options": ["bullets","shells","rockets","cells"]},
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

const CORE_FILES := [
	"res://main.tscn", "res://main_menu.tscn",
	"res://project.godot", "res://OpenHeart.gdextension",
]

# Property-панель для визуального редактора карты
const MAP_SCHEMAS := {
	"blocks":         [
		{"key":"shape","type":"enum","options":["box","ramp","stairs","cylinder"]},
		{"key":"pos","type":"json"},{"key":"size","type":"json"},
		{"key":"from","type":"json"},{"key":"to","type":"json"},
		{"key":"rot","type":"float"},{"key":"radius","type":"float"},
		{"key":"height","type":"float"},{"key":"tex","type":"str"},{"key":"uv","type":"float"},
	],
	"buildings":      [
		{"key":"pos","type":"json"},{"key":"size","type":"json"},
		{"key":"tex","type":"str"},{"key":"sign","type":"str"},
		{"key":"sign_side","type":"enum","options":["n","s","e","w"]},
	],
	"lights":         [
		{"key":"pos","type":"json"},{"key":"color","type":"json"},
		{"key":"energy","type":"float"},{"key":"range","type":"float"},
	],
	"props":          [{"key":"tex","type":"str"},{"key":"pos","type":"json"},{"key":"px","type":"float"}],
	"glows":          [{"key":"pos","type":"json"},{"key":"size","type":"json"},{"key":"tex","type":"str"},{"key":"emission","type":"json"},{"key":"uv","type":"float"}],
	"spawns_enemies": [{"key":"kind","type":"str"},{"key":"x","type":"float"},{"key":"z","type":"float"}],
	"spawns_items":   [{"key":"kind","type":"str"},{"key":"x","type":"float"},{"key":"z","type":"float"}],
}

# ────────────────────── ХОЛСТ КАРТЫ (inner class) ──────────────────────────────

class MapCanvas extends Control:
	var pan  := Vector2(500.0, 400.0)
	var zoom := 4.0

	var sel_layer := ""
	var sel_idx   := -1

	var _drag_active    := false
	var _drag_world_ofs := Vector2.ZERO
	var _pan_drag       := false
	var _pan_start      := Vector2.ZERO
	var _pan_origin     := Vector2.ZERO

	var blocks:        Array = []
	var buildings:     Array = []
	var lights:        Array = []
	var props:         Array = []
	var glows:         Array = []
	var spawn_enemies: Array = []
	var spawn_items:   Array = []

	signal selection_changed(layer: String, idx: int)
	signal data_modified()

	func _ready() -> void:
		mouse_filter = MOUSE_FILTER_STOP
		focus_mode   = FOCUS_ALL

	func w2c(x: float, z: float) -> Vector2:
		return Vector2(x * zoom + pan.x, z * zoom + pan.y)

	func c2w(p: Vector2) -> Vector2:
		return (p - pan) / zoom

	func reset_view() -> void:
		zoom = 4.0; pan = size * 0.5; queue_redraw()

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Color(0.07, 0.05, 0.09))
		_draw_grid(); _draw_boundary()
		_draw_glows_v(); _draw_blocks_v(); _draw_buildings_v()
		_draw_lights_v(); _draw_props_v(); _draw_spawns_v()

	func _draw_grid() -> void:
		var step := 10.0 * zoom
		var ox := fmod(pan.x, step); var oy := fmod(pan.y, step)
		var col := Color(0.14, 0.11, 0.17)
		var x := ox
		while x < size.x: draw_line(Vector2(x, 0), Vector2(x, size.y), col, 0.5); x += step
		var y := oy
		while y < size.y: draw_line(Vector2(0, y), Vector2(size.x, y), col, 0.5); y += step
		draw_line(Vector2(pan.x, 0), Vector2(pan.x, size.y), Color(0.30, 0.20, 0.40), 1.0)
		draw_line(Vector2(0, pan.y), Vector2(size.x, pan.y), Color(0.30, 0.20, 0.40), 1.0)

	func _draw_boundary() -> void:
		var tl := w2c(-100.0, -100.0); var br := w2c(100.0, 100.0)
		draw_rect(Rect2(tl, br-tl), Color(0.20,0.15,0.28,0.10), true)
		draw_rect(Rect2(tl, br-tl), Color(0.60,0.40,0.90,0.60), false, 1.5)

	func _item_rect(item: Dictionary, two_d: bool) -> Rect2:
		var shape: String = item.get("shape", "box")
		if shape in ["ramp","stairs"] and item.has("from") and item.has("to"):
			var fr = item["from"]; var to = item["to"]
			if fr is Array and to is Array and fr.size() >= 3 and to.size() >= 3:
				var tl := w2c(minf(float(fr[0]),float(to[0])), minf(float(fr[2]),float(to[2])))
				var br := w2c(maxf(float(fr[0]),float(to[0])), maxf(float(fr[2]),float(to[2])))
				return Rect2(tl, (br-tl).abs() + Vector2(1,1))
		var pos = item.get("pos", null); var sz = item.get("size", null)
		if not (pos is Array and sz is Array): return Rect2()
		var px: float; var pz: float
		if two_d and pos.size() >= 2: px = float(pos[0]); pz = float(pos[1])
		elif pos.size() >= 3:         px = float(pos[0]); pz = float(pos[2])
		else: return Rect2()
		var sw: float = float(sz[0]) if sz.size() >= 1 else 1.0
		var sd: float = float(sz[2]) if sz.size() >= 3 else (float(sz[1]) if sz.size() >= 2 else 1.0)
		var tl := w2c(px - sw*0.5, pz - sd*0.5); var br := w2c(px + sw*0.5, pz + sd*0.5)
		return Rect2(tl, (br-tl).abs() + Vector2(1,1))

	func _sel(layer: String, i: int) -> bool: return sel_layer == layer and sel_idx == i

	func _draw_blocks_v() -> void:
		for i in blocks.size():
			var b: Dictionary = blocks[i]; var s := _sel("blocks", i)
			var shape: String = b.get("shape","box")
			var fill  := Color(0.25,0.42,0.85, 0.9 if s else 0.70)
			var bdr   := Color(1.0,1.0,0.5) if s else Color(0.50,0.70,1.00)
			if shape == "cylinder":
				var pos = b.get("pos",null)
				if not (pos is Array and pos.size() >= 3): continue
				var cp := w2c(float(pos[0]),float(pos[2]))
				var rp: float = float(b.get("radius",1.0)) * zoom
				draw_circle(cp, rp, fill); draw_arc(cp, rp, 0, TAU, 32, bdr, 2.5 if s else 1.0)
				if s: draw_arc(cp, rp+3, 0, TAU, 32, Color(1,1,0.5,0.4), 1.0)
			else:
				var r := _item_rect(b, false)
				if r.size.x > 0.5: draw_rect(r, fill); draw_rect(r, bdr, false, 2.5 if s else 1.0)

	func _draw_buildings_v() -> void:
		for i in buildings.size():
			var b: Dictionary = buildings[i]; var s := _sel("buildings", i)
			var fill  := Color(0.85,0.44,0.10, 0.9 if s else 0.70)
			var bdr   := Color(1.0,1.0,0.5) if s else Color(1.00,0.65,0.20)
			var r := _item_rect(b, true)
			if r.size.x > 0.5:
				draw_rect(r, fill); draw_rect(r, bdr, false, 2.5 if s else 1.0)
				if s: draw_rect(r.grow(3), Color(1,1,0.5,0.3), false, 1.0)
				if zoom >= 3.0 and r.size.x > 30:
					draw_string(ThemeDB.fallback_font, r.position+Vector2(3,13),
						b.get("sign",""), HORIZONTAL_ALIGNMENT_LEFT, r.size.x-6, 9, Color(1,1,1,0.7))

	func _draw_glows_v() -> void:
		for i in glows.size():
			var g: Dictionary = glows[i]; var s := _sel("glows", i)
			var em = g.get("emission", [0.9,0.3,0.5])
			var fill := Color(
				float(em[0]) if em is Array and em.size()>0 else 0.9,
				float(em[1]) if em is Array and em.size()>1 else 0.3,
				float(em[2]) if em is Array and em.size()>2 else 0.5,
				0.65 if s else 0.35)
			var r := _item_rect(g, false)
			if r.size.x > 0.5: draw_rect(r, fill); draw_rect(r, fill.lightened(0.4), false, 2.0 if s else 1.0)

	func _draw_lights_v() -> void:
		for i in lights.size():
			var l: Dictionary = lights[i]; var s := _sel("lights", i)
			var pos = l.get("pos",null)
			if not (pos is Array and pos.size() >= 3): continue
			var ca = l.get("color",[1.0,0.85,0.3])
			var lcol := Color(
				float(ca[0]) if ca is Array and ca.size()>0 else 1.0,
				float(ca[1]) if ca is Array and ca.size()>1 else 0.85,
				float(ca[2]) if ca is Array and ca.size()>2 else 0.3, 0.25)
			var cp := w2c(float(pos[0]),float(pos[2]))
			var rp: float = float(l.get("range",6.0)) * zoom
			draw_circle(cp, rp, lcol)
			draw_arc(cp, maxf(rp,3), 0, TAU, 32, Color(1.0,1.0,0.5) if s else Color(1,0.9,0.3), 2.0 if s else 1.0)

	func _draw_props_v() -> void:
		for i in props.size():
			var p: Dictionary = props[i]; var pos = p.get("pos",null)
			if not (pos is Array and pos.size() >= 3): continue
			var cp := w2c(float(pos[0]),float(pos[2]))
			if cp.x < -12 or cp.x > size.x+12 or cp.y < -12 or cp.y > size.y+12: continue
			var s := _sel("props", i)
			draw_circle(cp, 5.0 if s else 4.0, Color(0.25,0.90,0.35,0.85))
			if s: draw_arc(cp, 8, 0, TAU, 16, Color(1,1,0.5), 2.0)
			if zoom >= 5.0:
				draw_string(ThemeDB.fallback_font, cp+Vector2(5,-4),
					p.get("tex",""), HORIZONTAL_ALIGNMENT_LEFT, 80, 9, Color(0.8,1,0.8,0.7))

	func _draw_spawns_v() -> void:
		for i in spawn_enemies.size():
			var sp: Dictionary = spawn_enemies[i]
			var cp := w2c(float(sp.get("x",0)),float(sp.get("z",0)))
			if cp.x < -12 or cp.x > size.x+12 or cp.y < -12 or cp.y > size.y+12: continue
			var s := _sel("spawns_enemies", i)
			var pts := PackedVector2Array([cp+Vector2(0,-8), cp+Vector2(7,6), cp+Vector2(-7,6)])
			draw_colored_polygon(pts, Color(0.90,0.18,0.18,0.85))
			if s: draw_polyline(PackedVector2Array([pts[0],pts[1],pts[2],pts[0]]), Color(1,1,0.5), 2.0)
			if zoom >= 4.0:
				draw_string(ThemeDB.fallback_font, cp+Vector2(8,4), sp.get("kind",""),
					HORIZONTAL_ALIGNMENT_LEFT, 80, 9, Color(1,0.6,0.6,0.8))
		for i in spawn_items.size():
			var sp: Dictionary = spawn_items[i]
			var cp := w2c(float(sp.get("x",0)),float(sp.get("z",0)))
			if cp.x < -12 or cp.x > size.x+12 or cp.y < -12 or cp.y > size.y+12: continue
			var s := _sel("spawns_items", i)
			draw_circle(cp, 5.0, Color(0.25,0.80,0.95,0.85))
			if s: draw_arc(cp, 8, 0, TAU, 16, Color(1,1,0.5), 2.0)

	func _gui_input(event: InputEvent) -> void:
		if event is InputEventMouseButton:
			var mb: InputEventMouseButton = event
			match mb.button_index:
				MOUSE_BUTTON_MIDDLE:
					_pan_drag = mb.pressed
					if mb.pressed: _pan_start = mb.position; _pan_origin = pan
				MOUSE_BUTTON_LEFT:
					if mb.pressed:
						var hit := _hit_test(mb.position)
						if hit.is_empty():
							sel_layer = ""; sel_idx = -1
							emit_signal("selection_changed", "", -1)
						else:
							sel_layer = hit["layer"]; sel_idx = hit["idx"]
							emit_signal("selection_changed", sel_layer, sel_idx)
							_drag_active = true
							_drag_world_ofs = _get_xz(sel_layer, sel_idx) - c2w(mb.position)
						queue_redraw()
					else:
						_drag_active = false
				MOUSE_BUTTON_WHEEL_UP:   _zoom_at(mb.position, 1.15)
				MOUSE_BUTTON_WHEEL_DOWN: _zoom_at(mb.position, 0.87)
		elif event is InputEventMouseMotion:
			var mm: InputEventMouseMotion = event
			if _pan_drag:
				pan = _pan_origin + (mm.position - _pan_start); queue_redraw()
			elif _drag_active and sel_idx >= 0:
				var wp := c2w(mm.position) + _drag_world_ofs
				_set_xz(sel_layer, sel_idx, snappedf(wp.x,0.5), snappedf(wp.y,0.5))
				emit_signal("data_modified"); queue_redraw()
		elif event is InputEventKey and event.pressed:
			if event.keycode == KEY_DELETE and sel_idx >= 0:
				var arr := _arr(sel_layer)
				if sel_idx < arr.size():
					arr.remove_at(sel_idx); sel_idx = -1
					selection_changed.emit("", -1); data_modified.emit(); queue_redraw()
			elif event.keycode == KEY_R: reset_view()

	func _zoom_at(c: Vector2, f: float) -> void:
		var nz := clampf(zoom*f, 0.5, 30.0)
		pan = c + (pan-c) * (nz/zoom); zoom = nz; queue_redraw()

	func _hit_test(cp: Vector2) -> Dictionary:
		for i in range(spawn_items.size()-1,-1,-1):
			if cp.distance_to(w2c(float(spawn_items[i].get("x",0)),float(spawn_items[i].get("z",0)))) <= 10: return {"layer":"spawns_items","idx":i}
		for i in range(spawn_enemies.size()-1,-1,-1):
			if cp.distance_to(w2c(float(spawn_enemies[i].get("x",0)),float(spawn_enemies[i].get("z",0)))) <= 10: return {"layer":"spawns_enemies","idx":i}
		for i in range(props.size()-1,-1,-1):
			var pos = props[i].get("pos",null)
			if pos is Array and pos.size()>=3 and cp.distance_to(w2c(float(pos[0]),float(pos[2]))) <= 10: return {"layer":"props","idx":i}
		for i in range(lights.size()-1,-1,-1):
			var pos = lights[i].get("pos",null)
			if pos is Array and pos.size()>=3 and cp.distance_to(w2c(float(pos[0]),float(pos[2]))) <= 12: return {"layer":"lights","idx":i}
		for i in range(buildings.size()-1,-1,-1):
			if _item_rect(buildings[i], true).has_point(cp): return {"layer":"buildings","idx":i}
		for i in range(blocks.size()-1,-1,-1):
			var b: Dictionary = blocks[i]
			if b.get("shape","box") == "cylinder":
				var pos = b.get("pos",null)
				if pos is Array and pos.size()>=3 and cp.distance_to(w2c(float(pos[0]),float(pos[2]))) <= float(b.get("radius",1.0))*zoom+4: return {"layer":"blocks","idx":i}
			elif _item_rect(b, false).has_point(cp): return {"layer":"blocks","idx":i}
		for i in range(glows.size()-1,-1,-1):
			if _item_rect(glows[i], false).has_point(cp): return {"layer":"glows","idx":i}
		return {}

	func _arr(layer: String) -> Array:
		match layer:
			"blocks":         return blocks
			"buildings":      return buildings
			"lights":         return lights
			"props":          return props
			"glows":          return glows
			"spawns_enemies": return spawn_enemies
			"spawns_items":   return spawn_items
		return []

	func _get_xz(layer: String, idx: int) -> Vector2:
		var arr := _arr(layer)
		if idx >= arr.size(): return Vector2.ZERO
		var item: Dictionary = arr[idx]
		if layer in ["spawns_enemies","spawns_items"]: return Vector2(float(item.get("x",0)),float(item.get("z",0)))
		var pos = item.get("pos",null)
		if not pos is Array: return Vector2.ZERO
		if layer == "buildings" and pos.size()>=2: return Vector2(float(pos[0]),float(pos[1]))
		if pos.size()>=3: return Vector2(float(pos[0]),float(pos[2]))
		return Vector2.ZERO

	func _set_xz(layer: String, idx: int, x: float, z: float) -> void:
		var arr := _arr(layer)
		if idx >= arr.size(): return
		var item: Dictionary = arr[idx]
		if layer in ["spawns_enemies","spawns_items"]: item["x"] = x; item["z"] = z; return
		if not item.has("pos"): item["pos"] = []
		var pos: Array = item["pos"]
		if layer == "buildings":
			if pos.size()<2: pos.resize(2); pos[0] = x; pos[1] = z
		else:
			if pos.size()<3: pos.resize(3)
			pos[0] = x
			if pos.size()>=3: pos[2] = z


# ────────────────────── ХОЛСТ ТЕКСТУР (inner class) ───────────────────────────

class TexCanvas extends Control:
	var image:   Image
	var texture: ImageTexture
	var zoom    := 8
	var pan     := Vector2.ZERO
	var tool    := "pencil"
	var paint_color := Color.WHITE
	var _painting   := false

	signal color_picked(c: Color)

	func _ready() -> void:
		mouse_filter = MOUSE_FILTER_STOP; focus_mode = FOCUS_ALL

	func load_img(img: Image) -> void:
		image = img.duplicate()
		texture = ImageTexture.create_from_image(image)
		zoom = clampi(256 / maxi(image.get_width(), image.get_height()), 1, 16)
		_center(); queue_redraw()

	func _center() -> void:
		if not image: return
		pan = size*0.5 - Vector2(image.get_width()*zoom, image.get_height()*zoom)*0.5

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Color(0.10,0.08,0.13))
		if not image:
			draw_string(ThemeDB.fallback_font, size*0.5-Vector2(70,8),
				"Загрузи PNG кнопкой 📂", HORIZONTAL_ALIGNMENT_LEFT, -1, 13, Color(0.4,0.4,0.4))
			return
		var w := image.get_width(); var h := image.get_height()
		var cw := w*zoom; var ch := h*zoom
		var cs := maxi(zoom,4); var rp := 0; var ry := 0
		while ry < ch:
			var cp := rp; var rx := 0
			while rx < cw:
				draw_rect(Rect2(pan+Vector2(rx,ry), Vector2(mini(cs,cw-rx),mini(cs,ch-ry))),
					Color(0.45,0.45,0.45) if cp%2==0 else Color(0.60,0.60,0.60))
				cp += 1; rx += cs
			rp += 1; ry += cs
		draw_texture_rect(texture, Rect2(pan, Vector2(cw,ch)), false)
		if zoom >= 4:
			for gy in h+1: draw_line(pan+Vector2(0,gy*zoom), pan+Vector2(cw,gy*zoom), Color(0,0,0,0.25), 0.5)
			for gx in w+1: draw_line(pan+Vector2(gx*zoom,0), pan+Vector2(gx*zoom,ch), Color(0,0,0,0.25), 0.5)
		draw_rect(Rect2(pan, Vector2(cw,ch)), Color(0.5,0.5,0.8,0.5), false, 1.0)

	func _px(cp: Vector2) -> Vector2i:
		return Vector2i(int((cp.x-pan.x)/zoom), int((cp.y-pan.y)/zoom))

	func _paint(cp: Vector2) -> void:
		if not image: return
		var p := _px(cp)
		if p.x<0 or p.x>=image.get_width() or p.y<0 or p.y>=image.get_height(): return
		match tool:
			"pencil": image.set_pixel(p.x, p.y, paint_color); _flush()
			"eraser": image.set_pixel(p.x, p.y, Color(0,0,0,0)); _flush()
			"fill":
				var tgt := image.get_pixel(p.x, p.y)
				if not tgt.is_equal_approx(paint_color): _flood(p.x,p.y,tgt,paint_color); _flush()
			"pick": emit_signal("color_picked", image.get_pixel(p.x,p.y))

	func _flood(sx:int, sy:int, tgt:Color, fill:Color) -> void:
		var w := image.get_width(); var h := image.get_height()
		var stack := [[sx,sy]]; var vis := {}; var lim := w*h; var n := 0
		while stack.size()>0 and n<lim:
			n += 1; var pos: Array = stack.pop_back(); var x:int=pos[0]; var y:int=pos[1]
			if x<0 or x>=w or y<0 or y>=h: continue
			var k := x+y*w
			if vis.has(k): continue
			vis[k] = true
			if not image.get_pixel(x,y).is_equal_approx(tgt): continue
			image.set_pixel(x,y,fill)
			stack.push_back([x+1,y]); stack.push_back([x-1,y])
			stack.push_back([x,y+1]); stack.push_back([x,y-1])

	func _flush() -> void:
		if texture: texture.update(image); queue_redraw()

	func _gui_input(event: InputEvent) -> void:
		if event is InputEventMouseButton:
			var mb: InputEventMouseButton = event
			if mb.button_index == MOUSE_BUTTON_LEFT:
				_painting = mb.pressed
				if mb.pressed: _paint(mb.position)
			elif mb.button_index == MOUSE_BUTTON_WHEEL_UP:
				var oz := zoom; zoom = mini(zoom*2, 64)
				if zoom!=oz: pan = mb.position+(pan-mb.position)*(float(zoom)/oz); queue_redraw()
			elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				var oz := zoom; zoom = maxi(zoom/2, 1)
				if zoom!=oz: pan = mb.position+(pan-mb.position)*(float(zoom)/oz); queue_redraw()
		elif event is InputEventMouseMotion:
			var mm: InputEventMouseMotion = event
			if _painting and (mm.button_mask & MOUSE_BUTTON_MASK_LEFT): _paint(mm.position)


# ─────────────────────────── СОСТОЯНИЕ ────────────────────────────────────────

# Вкладка «Данные»
var preset_id      := "core"
var category       := "Оружие"
var current_map    := "hub.json"   # файл в maps/, который правят категории "maps/*" и холст
var file_cache     := {}
var synthetic      := {}   # rel → true: файла нет на диске, в кэше — дефолт (не сохранять,
                           # пока пользователь его не тронул; отсутствие npcs.json — семантика!)
var dirty          := false
var preset_pick:   OptionButton
var map_pick:      OptionButton
var cat_list:      ItemList
var rec_list:      ItemList
var rec_map: Array[int] = []       # строка списка → индекс записи (фильтр поиска)
var search_edit:   LineEdit
var form_box:      VBoxContainer
var status:        Label
var lock_btn:      Button
var new_preset_edit: LineEdit

# Вкладка «ИИ-текстуры» (tools/aigen.py)
var gen_type:  OptionButton
var gen_id:    LineEdit
var gen_desc:  TextEdit
var gen_prompt_preview: Label
var gen_status: Label
var gen_preview: TextureRect
var gen_btn:   Button
var gen_thread: Thread
var gen_templates := {}

# Вкладка «Карта»
var map_canvas:    MapCanvas
var map_prop_box:  VBoxContainer
var map_status:    Label
var _map_ref:      Dictionary = {}

# Вкладка «Текстуры»
var tex_canvas:   TexCanvas
var tex_path_lbl: LineEdit
var tex_status:   Label
var tex_picker:   ColorPickerButton
var tex_cur_path  := ""

# Вкладка «Анимации»
var anim_path_lbl:  LineEdit
var anim_sheet_tr:  TextureRect
var anim_frame_lst: ItemList
var anim_preview:   TextureRect
var anim_overlay:   Control
var anim_status:    Label
var anim_fw:        SpinBox
var anim_fh:        SpinBox
var anim_fps:       SpinBox
var anim_frames: Array[Dictionary] = []
var anim_sheet_img: Image
var anim_sheet_tex: Texture2D
var anim_timer := 0.0
var anim_fidx  := 0
var anim_play  := false

# ────────────────────────── LIFECYCLE ─────────────────────────────────────────

func _ready() -> void:
	name = "OpenHeartEditor"
	set_anchors_preset(Control.PRESET_FULL_RECT)
	_build_ui(); _scan_presets(); _scan_maps(); _load_category()

func _exit_tree() -> void:
	# Плагин выключают/перезагружают: дождаться потока генерации, иначе Godot
	# ругается на Thread без wait_to_finish, а колбэк прилетит в мёртвый узел.
	if gen_thread != null:
		gen_thread.wait_to_finish()
		gen_thread = null

func _process(delta: float) -> void:
	if anim_play and anim_frames.size() > 0:
		anim_timer += delta
		var fps := anim_fps.value if anim_fps else 8.0
		if anim_timer >= 1.0 / maxf(fps, 1.0):
			anim_timer = 0.0
			anim_fidx = (anim_fidx + 1) % anim_frames.size()
			_anim_show(anim_fidx)

# ──────────────────────────── UI ROOT ─────────────────────────────────────────

func _build_ui() -> void:
	var tabs := TabContainer.new()
	tabs.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(tabs)
	var p0 := _mk_pane("Данные");   tabs.add_child(p0); _build_data_tab(p0)
	var p1 := _mk_pane("Карта");    tabs.add_child(p1); _build_map_tab(p1)
	var p2 := _mk_pane("Текстуры"); tabs.add_child(p2); _build_tex_tab(p2)
	var p3 := _mk_pane("Анимации"); tabs.add_child(p3); _build_anim_tab(p3)
	var p4 := _mk_pane("🎨 ИИ");    tabs.add_child(p4); _build_gen_tab(p4)

func _mk_pane(n: String) -> Control:
	var c := Control.new(); c.name = n; c.set_anchors_preset(Control.PRESET_FULL_RECT); return c

# ──────────────────── ВКЛАДКА «ДАННЫЕ» ────────────────────────────────────────

func _build_data_tab(parent: Control) -> void:
	var root := VBoxContainer.new(); root.set_anchors_preset(Control.PRESET_FULL_RECT); parent.add_child(root)
	var top := HBoxContainer.new(); root.add_child(top)
	top.add_child(_mk_lbl("Пресет:"))
	preset_pick = OptionButton.new(); preset_pick.item_selected.connect(_on_preset_sel); top.add_child(preset_pick)
	top.add_child(_mk_lbl("Карта:"))
	map_pick = OptionButton.new()
	map_pick.tooltip_text = "Файл из maps/, который редактируют категории «Карта: …» и холст"
	map_pick.item_selected.connect(_on_map_pick); top.add_child(map_pick)
	new_preset_edit = LineEdit.new(); new_preset_edit.placeholder_text = "id нового…"; new_preset_edit.custom_minimum_size.x = 140; top.add_child(new_preset_edit)
	var np := Button.new(); np.text = "Создать копию"; np.pressed.connect(_on_new_preset); top.add_child(np)
	top.add_spacer(false)
	var sb := Button.new(); sb.text = "💾 Сохранить"; sb.pressed.connect(_save_all); top.add_child(sb)
	lock_btn = Button.new(); lock_btn.toggle_mode = true; lock_btn.toggled.connect(_on_lock_tog); top.add_child(lock_btn)
	_refresh_lock()
	status = _mk_lbl(""); status.modulate = Color(1,0.7,0.9); top.add_child(status)

	var split := HSplitContainer.new(); split.size_flags_vertical = Control.SIZE_EXPAND_FILL; root.add_child(split)
	var left := VBoxContainer.new(); left.custom_minimum_size.x = 380; split.add_child(left)
	left.add_child(_mk_lbl("Категория"))
	cat_list = ItemList.new(); cat_list.custom_minimum_size.y = 220; cat_list.max_columns = 1
	for k in SCHEMAS.keys(): cat_list.add_item(k)
	cat_list.item_selected.connect(_on_cat_sel); left.add_child(cat_list)
	left.add_child(_mk_lbl("Записи"))
	search_edit = LineEdit.new(); search_edit.placeholder_text = "🔎 поиск…"
	search_edit.clear_button_enabled = true
	search_edit.text_changed.connect(func(_t): _refresh_recs(_sel_idx()))
	left.add_child(search_edit)
	rec_list = ItemList.new(); rec_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	rec_list.custom_minimum_size.y = 120; rec_list.max_columns = 1
	rec_list.item_selected.connect(_on_rec_sel); left.add_child(rec_list)
	var crud := HBoxContainer.new(); left.add_child(crud)
	var ba := Button.new(); ba.text = "＋"; ba.pressed.connect(_on_add)
	var bd := Button.new(); bd.text = "⧉"; bd.pressed.connect(_on_dup)
	var bx := Button.new(); bx.text = "🗑"; bx.pressed.connect(_on_del)
	crud.add_child(ba); crud.add_child(bd); crud.add_child(bx)

	var scroll := ScrollContainer.new(); scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL; scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL; split.add_child(scroll)
	form_box = VBoxContainer.new(); form_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL; scroll.add_child(form_box)

# ──────────────────── ВКЛАДКА «КАРТА» ─────────────────────────────────────────

func _build_map_tab(parent: Control) -> void:
	var root := VBoxContainer.new(); root.set_anchors_preset(Control.PRESET_FULL_RECT); parent.add_child(root)
	var bar := HBoxContainer.new(); root.add_child(bar)
	var rl := Button.new(); rl.text = "↺ Загрузить"; rl.pressed.connect(_map_load); bar.add_child(rl)
	var sv := Button.new(); sv.text = "💾 Сохранить"; sv.pressed.connect(_map_save); bar.add_child(sv)
	var rs := Button.new(); rs.text = "⊡ Сброс"; rs.pressed.connect(func(): if map_canvas: map_canvas.reset_view()); bar.add_child(rs)
	bar.add_spacer(false)
	for pair in [["blocks","Блок"],["buildings","Здание"],["lights","Свет"],["props","Проп"],["glows","Глоу"],["spawns_enemies","Враг"],["spawns_items","Лут"]]:
		var btn := Button.new(); btn.text = "+ %s" % pair[1]; btn.pressed.connect(_map_add.bind(pair[0])); bar.add_child(btn)
	map_status = _mk_lbl("← Загрузи карту"); map_status.modulate = Color(0.7,1,0.8); bar.add_child(map_status)

	var split := HSplitContainer.new(); split.size_flags_vertical = Control.SIZE_EXPAND_FILL; root.add_child(split)
	map_canvas = MapCanvas.new(); map_canvas.size_flags_horizontal = Control.SIZE_EXPAND_FILL; map_canvas.size_flags_vertical = Control.SIZE_EXPAND_FILL
	map_canvas.selection_changed.connect(_on_map_sel)
	map_canvas.data_modified.connect(func(): dirty = true; _touch("maps/%s" % current_map))
	split.add_child(map_canvas)
	var rscroll := ScrollContainer.new(); rscroll.custom_minimum_size.x = 320; rscroll.size_flags_vertical = Control.SIZE_EXPAND_FILL; split.add_child(rscroll)
	map_prop_box = VBoxContainer.new(); map_prop_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL; rscroll.add_child(map_prop_box)
	map_prop_box.add_child(_mk_lbl("← Выбери объект на холсте"))

func _map_load() -> void:
	var rel := "maps/%s" % current_map   # холст следует переключателю карт
	if not file_cache.has(rel):
		var p = _read_json("%s/%s" % [_preset_root(), rel])
		file_cache[rel] = p if p != null else {}
	_map_ref = file_cache[rel]
	for k in ["blocks","buildings","lights","props","glows"]:
		if not _map_ref.has(k): _map_ref[k] = []
	if not _map_ref.has("spawns"): _map_ref["spawns"] = {}
	if not _map_ref["spawns"].has("spawn_enemies"): _map_ref["spawns"]["spawn_enemies"] = []
	if not _map_ref["spawns"].has("spawn_items"):   _map_ref["spawns"]["spawn_items"] = []
	map_canvas.blocks        = _map_ref["blocks"]
	map_canvas.buildings     = _map_ref["buildings"]
	map_canvas.lights        = _map_ref["lights"]
	map_canvas.props         = _map_ref["props"]
	map_canvas.glows         = _map_ref["glows"]
	map_canvas.spawn_enemies = _map_ref["spawns"]["spawn_enemies"]
	map_canvas.spawn_items   = _map_ref["spawns"]["spawn_items"]
	map_canvas.reset_view()
	map_status.text = "Загружено: %d блоков, %d зданий" % [map_canvas.blocks.size(), map_canvas.buildings.size()]

func _map_save() -> void: _save_all(); map_status.text = "Сохранено ✓"

func _map_add(layer: String) -> void:
	if _map_ref.is_empty(): _map_load()
	var arr := map_canvas._arr(layer)
	var blank: Dictionary
	match layer:
		"blocks":         blank = {"shape":"box","pos":[0.0,0.0,0.0],"size":[4.0,3.0,4.0],"tex":"wall_main","uv":2.0}
		"buildings":      blank = {"pos":[0.0,0.0],"size":[10.0,5.0,8.0],"tex":"wall_main","sign":"","sign_side":"s"}
		"lights":         blank = {"pos":[0.0,2.0,0.0],"color":[1.0,0.8,0.3],"energy":1.0,"range":8.0}
		"props":          blank = {"tex":"street_bench","pos":[0.0,0.0,0.0],"px":0.020}
		"glows":          blank = {"pos":[0.0,0.03,0.0],"size":[4.0,0.06,4.0],"tex":"liquid_pink","emission":[0.9,0.3,0.6],"uv":2.0}
		"spawns_enemies": blank = {"kind":"grunt","x":0.0,"z":0.0}
		"spawns_items":   blank = {"kind":"medkit","x":0.0,"z":0.0}
		_: blank = {}
	arr.append(blank)
	dirty = true; _touch("maps/%s" % current_map)
	map_canvas.sel_layer = layer; map_canvas.sel_idx = arr.size()-1
	map_canvas.queue_redraw(); _on_map_sel(layer, arr.size()-1); dirty = true

func _on_map_sel(layer: String, idx: int) -> void:
	for c in map_prop_box.get_children(): c.queue_free()
	if layer.is_empty() or idx < 0: map_prop_box.add_child(_mk_lbl("← Выбери объект")); return
	var arr := map_canvas._arr(layer)
	if idx >= arr.size(): return
	var rec: Dictionary = arr[idx]
	var hdr := _mk_lbl("[%s] #%d" % [layer, idx]); hdr.add_theme_color_override("font_color", Color(1,0.85,0.4)); map_prop_box.add_child(hdr)
	for field in MAP_SCHEMAS.get(layer, []):
		var key: String = field["key"]
		if not rec.has(key): continue
		var row := HBoxContainer.new()
		var lbl := _mk_lbl(key); lbl.custom_minimum_size.x = 90; row.add_child(lbl)
		var ed := _make_field(field, rec, -1); ed.size_flags_horizontal = Control.SIZE_EXPAND_FILL; row.add_child(ed)
		map_prop_box.add_child(row)
	var db := Button.new(); db.text = "🗑 Удалить"
	db.pressed.connect(func():
		arr.remove_at(idx); map_canvas.sel_idx = -1; map_canvas.queue_redraw()
		_on_map_sel("", -1); dirty = true)
	map_prop_box.add_child(db)

# ──────────────────── ВКЛАДКА «ТЕКСТУРЫ» ──────────────────────────────────────

func _build_tex_tab(parent: Control) -> void:
	var root := VBoxContainer.new(); root.set_anchors_preset(Control.PRESET_FULL_RECT); parent.add_child(root)
	var bar := HBoxContainer.new(); root.add_child(bar)
	tex_path_lbl = LineEdit.new(); tex_path_lbl.placeholder_text = "res://assets/textures/wall_main.png"; tex_path_lbl.custom_minimum_size.x = 340; bar.add_child(tex_path_lbl)
	var opn := Button.new(); opn.text = "📂"; opn.pressed.connect(_tex_load); bar.add_child(opn)
	var sav := Button.new(); sav.text = "💾"; sav.pressed.connect(_tex_save); bar.add_child(sav)
	for pair in [["pencil","✏"],["eraser","◻"],["fill","🪣"],["pick","💧"]]:
		var btn := Button.new(); btn.text = pair[1]; btn.toggle_mode = true; btn.button_pressed = (pair[0]=="pencil")
		btn.pressed.connect(_set_tex_tool.bind(pair[0])); bar.add_child(btn)
	tex_picker = ColorPickerButton.new(); tex_picker.color = Color.WHITE; tex_picker.custom_minimum_size = Vector2(44,26)
	tex_picker.color_changed.connect(func(c: Color): if tex_canvas: tex_canvas.paint_color = c); bar.add_child(tex_picker)
	var zm := Button.new(); zm.text="−"; zm.pressed.connect(func(): if tex_canvas: tex_canvas.zoom=maxi(tex_canvas.zoom/2,1); tex_canvas.queue_redraw()); bar.add_child(zm)
	var zp := Button.new(); zp.text="＋"; zp.pressed.connect(func(): if tex_canvas: tex_canvas.zoom=mini(tex_canvas.zoom*2,64); tex_canvas.queue_redraw()); bar.add_child(zp)
	var zf := Button.new(); zf.text="⊡"; zf.pressed.connect(func(): if tex_canvas: tex_canvas._center(); tex_canvas.queue_redraw()); bar.add_child(zf)
	tex_status = _mk_lbl("Загрузи PNG"); tex_status.modulate = Color(0.7,1,0.8); bar.add_child(tex_status)

	tex_canvas = TexCanvas.new(); tex_canvas.size_flags_horizontal = Control.SIZE_EXPAND_FILL; tex_canvas.size_flags_vertical = Control.SIZE_EXPAND_FILL
	tex_canvas.color_picked.connect(func(c: Color): tex_picker.color = c; tex_canvas.paint_color = c); root.add_child(tex_canvas)

	var pal := HBoxContainer.new(); root.add_child(pal); pal.add_child(_mk_lbl("Палитра: "))
	for pc in [Color("#ff4fa3"),Color("#a855f7"),Color("#3b82f6"),Color("#10b981"),Color("#f59e0b"),Color("#ef4444"),Color("#6b7280"),Color("#1c0a2e"),Color.WHITE,Color.BLACK]:
		var cb := ColorRect.new(); cb.color=pc; cb.custom_minimum_size=Vector2(22,22)
		cb.gui_input.connect(func(ev: InputEvent):
			if ev is InputEventMouseButton and ev.button_index==MOUSE_BUTTON_LEFT and ev.pressed:
				tex_picker.color=pc; if tex_canvas: tex_canvas.paint_color=pc)
		pal.add_child(cb)

func _set_tex_tool(tn: String) -> void:
	if tex_canvas: tex_canvas.tool = tn

func _tex_load() -> void:
	var path := tex_path_lbl.text.strip_edges()
	if path.is_empty(): return
	var img := Image.load_from_file(ProjectSettings.globalize_path(path))
	if not img: tex_status.text = "Не открылся: %s" % path; tex_status.modulate = Color(1,0.5,0.5); return
	tex_canvas.load_img(img); tex_cur_path = path
	tex_status.text = "%s  (%dx%d)" % [path.get_file(), img.get_width(), img.get_height()]; tex_status.modulate = Color(0.6,1,0.7)

func _tex_save() -> void:
	if not tex_canvas or not tex_canvas.image: return
	var path := tex_cur_path if not tex_cur_path.is_empty() else tex_path_lbl.text.strip_edges()
	if path.is_empty(): return
	var err := tex_canvas.image.save_png(ProjectSettings.globalize_path(path))
	if err == OK: tex_status.text = "Сохранено ✓"; EditorInterface.get_resource_filesystem().scan()
	else: tex_status.text = "Ошибка %d" % err; tex_status.modulate = Color(1,0.5,0.5)

# ──────────────────── ВКЛАДКА «АНИМАЦИИ» ──────────────────────────────────────

func _build_anim_tab(parent: Control) -> void:
	var root := VBoxContainer.new(); root.set_anchors_preset(Control.PRESET_FULL_RECT); parent.add_child(root)
	var bar := HBoxContainer.new(); root.add_child(bar)
	anim_path_lbl = LineEdit.new(); anim_path_lbl.placeholder_text = "res://assets/sprites/weapons/weapon_00.png"; anim_path_lbl.custom_minimum_size.x = 320; bar.add_child(anim_path_lbl)
	var opn := Button.new(); opn.text = "📂 Лист"; opn.pressed.connect(_anim_load); bar.add_child(opn)
	bar.add_child(_mk_lbl("  W:")); anim_fw = SpinBox.new(); anim_fw.min_value=1; anim_fw.max_value=1024; anim_fw.value=96; anim_fw.value_changed.connect(func(_v): if anim_overlay: anim_overlay.queue_redraw()); bar.add_child(anim_fw)
	bar.add_child(_mk_lbl(" H:")); anim_fh = SpinBox.new(); anim_fh.min_value=1; anim_fh.max_value=1024; anim_fh.value=256; anim_fh.value_changed.connect(func(_v): if anim_overlay: anim_overlay.queue_redraw()); bar.add_child(anim_fh)
	bar.add_child(_mk_lbl(" FPS:")); anim_fps = SpinBox.new(); anim_fps.min_value=1; anim_fps.max_value=60; anim_fps.value=8; bar.add_child(anim_fps)
	anim_status = _mk_lbl("Загрузи спрайт-лист"); anim_status.modulate = Color(0.7,1,0.8); bar.add_child(anim_status)

	var split := HSplitContainer.new(); split.size_flags_vertical = Control.SIZE_EXPAND_FILL; root.add_child(split)
	var sheet_sc := ScrollContainer.new(); sheet_sc.size_flags_horizontal = Control.SIZE_EXPAND_FILL; sheet_sc.size_flags_vertical = Control.SIZE_EXPAND_FILL; split.add_child(sheet_sc)
	var sheet_con := Control.new(); sheet_con.custom_minimum_size = Vector2(512,512); sheet_sc.add_child(sheet_con)
	anim_sheet_tr = TextureRect.new(); anim_sheet_tr.set_anchors_preset(Control.PRESET_FULL_RECT)
	anim_sheet_tr.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED; anim_sheet_tr.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	anim_sheet_tr.texture_filter = TEXTURE_FILTER_NEAREST; sheet_con.add_child(anim_sheet_tr)
	anim_overlay = Control.new(); anim_overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	anim_overlay.mouse_filter = Control.MOUSE_FILTER_STOP
	anim_overlay.draw.connect(_anim_draw_ov.bind(anim_overlay)); anim_overlay.gui_input.connect(_anim_click); sheet_con.add_child(anim_overlay)

	var right := VBoxContainer.new(); right.custom_minimum_size.x = 260; split.add_child(right)
	right.add_child(_mk_lbl("Последовательность (кликай по листу):"))
	anim_frame_lst = ItemList.new(); anim_frame_lst.custom_minimum_size.y = 180; anim_frame_lst.size_flags_vertical = Control.SIZE_EXPAND_FILL; anim_frame_lst.max_columns = 1; right.add_child(anim_frame_lst)
	var acrud := HBoxContainer.new(); right.add_child(acrud)
	var rem := Button.new(); rem.text = "− Убрать"; rem.pressed.connect(_anim_rem); acrud.add_child(rem)
	var clr := Button.new(); clr.text = "✕ Очистить"; clr.pressed.connect(_anim_clr); acrud.add_child(clr)
	var pbar := HBoxContainer.new(); right.add_child(pbar)
	var pb := Button.new(); pb.text = "▶ Play"; pb.pressed.connect(func(): anim_play=true; anim_fidx=0; anim_timer=0.0); pbar.add_child(pb)
	var sb2 := Button.new(); sb2.text = "■ Stop"; sb2.pressed.connect(func(): anim_play=false); pbar.add_child(sb2)
	right.add_child(_mk_lbl("Предпросмотр:"))
	anim_preview = TextureRect.new(); anim_preview.custom_minimum_size = Vector2(128,128)
	anim_preview.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED; anim_preview.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	anim_preview.texture_filter = TEXTURE_FILTER_NEAREST; right.add_child(anim_preview)
	var cpb := Button.new(); cpb.text = "📋 Скопировать индексы"; cpb.pressed.connect(_anim_copy); right.add_child(cpb)

func _anim_load() -> void:
	var path := anim_path_lbl.text.strip_edges()
	if path.is_empty(): return
	var img := Image.load_from_file(ProjectSettings.globalize_path(path))
	if not img: anim_status.text = "Не открылся: %s" % path; anim_status.modulate = Color(1,0.5,0.5); return
	anim_sheet_img = img; anim_sheet_tex = ImageTexture.create_from_image(img)
	anim_sheet_tr.texture = anim_sheet_tex
	anim_fw.value = floori(img.get_width() / 4.0); anim_fh.value = img.get_height()
	if anim_sheet_tr.get_parent(): anim_sheet_tr.get_parent().custom_minimum_size = Vector2(img.get_width(), img.get_height())
	anim_frames.clear(); _anim_refresh_lst()
	if anim_overlay: anim_overlay.queue_redraw()
	anim_status.text = "%s  (%dx%d)" % [path.get_file(), img.get_width(), img.get_height()]; anim_status.modulate = Color(0.6,1,0.7)

func _anim_draw_ov(canvas: Control) -> void:
	if not anim_sheet_img: return
	var fw := int(anim_fw.value); var fh := int(anim_fh.value)
	var iw := anim_sheet_img.get_width(); var ih := anim_sheet_img.get_height()
	if fw<=0 or fh<=0: return
	var csz := canvas.size; var sc := minf(csz.x/iw, csz.y/ih)
	var off := (csz - Vector2(iw*sc, ih*sc))*0.5
	var cols := maxi(iw/fw,1); var rows := maxi(ih/fh,1)
	for c in cols+1: canvas.draw_line(Vector2(off.x+c*fw*sc,off.y), Vector2(off.x+c*fw*sc,off.y+ih*sc), Color(1,1,1,0.35), 1.0)
	for r in rows+1: canvas.draw_line(Vector2(off.x,off.y+r*fh*sc), Vector2(off.x+iw*sc,off.y+r*fh*sc), Color(1,1,1,0.35), 1.0)
	for frame in anim_frames:
		var fc: int = frame["col"]; var fr: int = frame["row"]
		canvas.draw_rect(Rect2(off.x+fc*fw*sc, off.y+fr*fh*sc, fw*sc, fh*sc), Color(1,1,0,0.22), true)
		canvas.draw_rect(Rect2(off.x+fc*fw*sc, off.y+fr*fh*sc, fw*sc, fh*sc), Color(1,1,0.5,0.8), false, 1.5)
	if sc*fw > 20:
		var idx := 0
		for r in rows:
			for c in cols:
				canvas.draw_string(ThemeDB.fallback_font, Vector2(off.x+c*fw*sc+2, off.y+r*fh*sc+13), str(idx), HORIZONTAL_ALIGNMENT_LEFT, -1, 10, Color(1,1,1,0.6))
				idx += 1

func _anim_click(event: InputEvent) -> void:
	if not event is InputEventMouseButton: return
	var mb: InputEventMouseButton = event
	if mb.button_index != MOUSE_BUTTON_LEFT or not mb.pressed: return
	if not anim_sheet_img: return
	var fw := int(anim_fw.value); var fh := int(anim_fh.value)
	var iw := anim_sheet_img.get_width(); var ih := anim_sheet_img.get_height()
	if fw<=0 or fh<=0: return
	var csz := anim_overlay.size; var sc := minf(csz.x/iw, csz.y/ih)
	var off := (csz - Vector2(iw*sc, ih*sc))*0.5
	var rel := mb.position - off
	if rel.x<0 or rel.y<0 or rel.x>iw*sc or rel.y>ih*sc: return
	var col := int(rel.x/(fw*sc)); var row := int(rel.y/(fh*sc))
	anim_frames.append({"col":col,"row":row}); _anim_refresh_lst(); _anim_show(anim_frames.size()-1)
	if anim_overlay: anim_overlay.queue_redraw()

func _anim_refresh_lst() -> void:
	anim_frame_lst.clear()
	var cols := maxi(int(anim_sheet_img.get_width() if anim_sheet_img else 1) / int(anim_fw.value if anim_fw else 1), 1)
	for i in anim_frames.size():
		var f: Dictionary = anim_frames[i]
		anim_frame_lst.add_item("#%d — кол%d ряд%d (idx %d)" % [i, f["col"], f["row"], f["row"]*cols+f["col"]])

func _anim_show(idx: int) -> void:
	if idx<0 or idx>=anim_frames.size() or not anim_sheet_img: return
	var f: Dictionary = anim_frames[idx]
	var fw := int(anim_fw.value); var fh := int(anim_fh.value)
	var sub := anim_sheet_img.get_region(Rect2(f["col"]*fw, f["row"]*fh, fw, fh))
	anim_preview.texture = ImageTexture.create_from_image(sub)

func _anim_rem() -> void:
	var sel := anim_frame_lst.get_selected_items()
	if sel.is_empty(): return
	anim_frames.remove_at(sel[0]); _anim_refresh_lst()
	if anim_overlay: anim_overlay.queue_redraw()

func _anim_clr() -> void:
	anim_frames.clear(); _anim_refresh_lst()
	if anim_overlay: anim_overlay.queue_redraw()

func _anim_copy() -> void:
	if not anim_sheet_img: return
	var cols := maxi(int(anim_sheet_img.get_width())/int(anim_fw.value), 1)
	var idx := []; for f in anim_frames: idx.append(f["row"]*cols+f["col"])
	DisplayServer.clipboard_set(JSON.stringify(idx))
	anim_status.text = "Скопировано: %s" % JSON.stringify(idx)

# ──────────────────── ДАННЫЕ: ПРЕСЕТЫ / ФАЙЛЫ ─────────────────────────────────

func _preset_root() -> String:
	# как content.rs::preset_base — сначала встроенные, затем user:// (моды)
	var res := "res://presets/%s" % preset_id
	if FileAccess.file_exists("%s/preset.json" % res): return res
	var user := "user://presets/%s" % preset_id
	if FileAccess.file_exists("%s/preset.json" % user): return user
	return res

## Файлы карт текущего пресета (maps/*.json) → переключатель в топ-баре.
func _scan_maps() -> void:
	map_pick.clear()
	var found: Array[String] = []
	var dir := DirAccess.open("%s/maps" % _preset_root())
	if dir:
		for f in dir.get_files():
			if f.ends_with(".json"): found.append(f)
	found.sort()
	for i in found.size():
		map_pick.add_item(found[i])
		if found[i] == current_map: map_pick.select(i)
	if not found.has(current_map):
		current_map = found[0] if found.size() > 0 else "hub.json"
		if found.size() > 0: map_pick.select(0)

func _on_map_pick(idx: int) -> void:
	current_map = map_pick.get_item_text(idx)
	if category.begins_with("Карта"): _refresh_recs(0)
	if map_canvas: _map_load()
	_set_st("Карта: %s" % current_map)

func _scan_presets() -> void:
	preset_pick.clear()
	var found: Array[String] = []
	for root in ["res://presets", "user://presets"]:   # user:// — моды
		var dir := DirAccess.open(root)
		if dir:
			for d in dir.get_directories():
				if not d.begins_with(".") and not found.has(d): found.append(d)
	found.sort()
	for i in found.size():
		preset_pick.add_item(found[i])
		if found[i] == preset_id: preset_pick.select(i)
	if not found.has(preset_id) and found.size() > 0:
		preset_id = found[0]; preset_pick.select(0)

func _on_preset_sel(idx: int) -> void:
	if dirty: _save_all()
	preset_id = preset_pick.get_item_text(idx)
	file_cache.clear(); synthetic.clear(); _map_ref.clear()
	_scan_maps(); _load_category()

func _on_new_preset() -> void:
	var nid := new_preset_edit.text.strip_edges()
	if nid.is_empty() or not nid.is_valid_filename(): _set_st("Некорректный id", false); return
	var dst := "res://presets/%s" % nid
	if DirAccess.dir_exists_absolute(dst): _set_st("Уже есть: %s" % nid, false); return
	_copy_dir(_preset_root(), dst)
	var mp := "%s/preset.json" % dst
	var info = _read_json(mp)
	if typeof(info)==TYPE_DICTIONARY: info["id"]=nid; info["name_ru"]=nid; _write_json(mp,info)
	preset_id = nid; file_cache.clear(); synthetic.clear(); _scan_presets(); _load_category()

func _copy_dir(src: String, dst: String) -> void:
	DirAccess.make_dir_recursive_absolute(dst)
	var dir := DirAccess.open(src)
	if not dir: return
	for f in dir.get_files(): dir.copy("%s/%s" % [src,f], "%s/%s" % [dst,f])
	for d in dir.get_directories(): _copy_dir("%s/%s" % [src,d], "%s/%s" % [dst,d])

func _read_json(path: String):
	var f := FileAccess.open(path, FileAccess.READ)
	if not f: return null
	return JSON.parse_string(f.get_as_text())

func _write_json(path: String, data) -> bool:
	var f := FileAccess.open(path, FileAccess.WRITE)
	if not f: return false
	f.store_string(JSON.stringify(data, "  ", false)); return true

func _schema() -> Dictionary: return SCHEMAS[category]

## Файл схемы с подстановкой выбранной карты ("maps/*" → maps/<current_map>).
func _schema_file() -> String:
	var f: String = _schema()["file"]
	return "maps/%s" % current_map if f == "maps/*" else f

## Содержимое файла пресета через общий кэш (несохранённые правки видны всем).
## Если файла нет — кэшируем дефолт и помечаем synthetic: _save_all его пропустит,
## пока запись реально не отредактируют (см. _touch).
func _cached_file(rel: String, default_root):
	if not file_cache.has(rel):
		var parsed = _read_json("%s/%s" % [_preset_root(), rel])
		if parsed == null:
			# У dungeon/loot «нет файла» = встроенные core-настройки: стартуем
			# от ПОЛНОЙ core-копии, чтобы правка одной секции не сохранила
			# усечённый файл (пустые kill_drops = «дропа нет вообще»).
			if rel == "dungeon.json" or rel == "loot.json":
				parsed = _read_json("res://presets/core/%s" % rel)
			synthetic[rel] = true
			if parsed == null:
				parsed = default_root
		file_cache[rel] = parsed
	return file_cache[rel]

## Файл rel реально изменён пользователем — можно сохранять на диск.
func _touch(rel: String) -> void:
	synthetic.erase(rel)

func _records() -> Array:
	var schema := _schema()
	var is_single: bool = schema.get("single", false)
	var node = _cached_file(_schema_file(),
		{} if (is_single or not (schema["root"] as Array).is_empty()) else [])
	var root: Array = schema["root"]
	if is_single:   # одна запись-словарь (preset.json, dungeon.settings, …)
		for k in root:
			if typeof(node) != TYPE_DICTIONARY: return []
			if not node.has(k): node[k] = {}
			node = node[k]
		return [node] if typeof(node) == TYPE_DICTIONARY else []
	for i in root.size():
		if typeof(node) != TYPE_DICTIONARY: return []
		# недостающую секцию создаём в кэше — иначе «Добавить» пишет в пустоту
		if not node.has(root[i]): node[root[i]] = [] if i == root.size()-1 else {}
		node = node[root[i]]
	return node if typeof(node)==TYPE_ARRAY else []

# ──────────────────── ДАННЫЕ: СПИСКИ ─────────────────────────────────────────

func _on_cat_sel(idx: int) -> void: category = cat_list.get_item_text(idx); _load_category()

func _on_rec_sel(row: int) -> void:
	if row >= 0 and row < rec_map.size(): _build_form(rec_map[row])

func _load_category() -> void:
	for i in cat_list.item_count:
		if cat_list.get_item_text(i) == category: cat_list.select(i); break
	_refresh_recs(0)

## sel — индекс ЗАПИСИ (не строки списка); rec_map хранит фильтр поиска.
func _refresh_recs(sel: int) -> void:
	rec_list.clear()
	rec_map.clear()
	var recs := _records()
	var filter := search_edit.text.strip_edges().to_lower() if search_edit else ""
	for i in recs.size():
		var title := _rec_title(recs[i])
		if filter.is_empty() or filter in title.to_lower():
			rec_list.add_item(title)
			rec_map.append(i)
	if rec_map.is_empty(): _clear_form(); return
	sel = clampi(sel, 0, recs.size()-1)
	var row := rec_map.find(sel)
	if row < 0: row = 0
	rec_list.select(row); _build_form(rec_map[row])

func _rec_title(r) -> String:
	if typeof(r) != TYPE_DICTIONARY: return str(r)
	var id   = r.get("id", r.get("kind", r.get("shape", r.get("tex", "запись"))))
	var name = r.get("name_ru", r.get("name", r.get("title_ru", "")))
	return "%s — %s" % [id, name] if name else str(id)

## Индекс выбранной ЗАПИСИ (через rec_map — список может быть отфильтрован).
func _sel_idx() -> int:
	var sel := rec_list.get_selected_items()
	if sel.size() == 0 or sel[0] >= rec_map.size(): return -1
	return rec_map[sel[0]]

func _clear_form() -> void:
	for c in form_box.get_children(): c.queue_free()

func _build_form(idx: int) -> void:
	_clear_form()
	var recs := _records()
	if idx < 0 or idx >= recs.size(): return
	var rec: Dictionary = recs[idx]
	for field in _schema()["fields"]:
		var row := HBoxContainer.new()
		var lbl := _mk_lbl(field["key"]); lbl.custom_minimum_size.x = 140; row.add_child(lbl)
		var ed := _make_field(field, rec, idx); ed.size_flags_horizontal = Control.SIZE_EXPAND_FILL; row.add_child(ed)
		form_box.add_child(row)

# ──────────────────── ДАННЫЕ: РЕДАКТОР ПОЛЯ ──────────────────────────────────

func _make_field(field: Dictionary, rec: Dictionary, rec_idx: int) -> Control:
	var key: String = field["key"]; var t: String = field["type"]; var val = rec.get(key)
	match t:
		"str":
			var e := LineEdit.new(); e.text = str(val) if val!=null else ""
			e.text_changed.connect(func(s): rec[key]=s; if rec_idx>=0: _mark(rec_idx)); return e
		"text":
			var e := TextEdit.new(); e.custom_minimum_size.y = 56; e.text = str(val) if val!=null else ""
			e.text_changed.connect(func(): rec[key]=e.text; if rec_idx>=0: _mark(rec_idx)); return e
		"float":
			# шаг 0.001: значения вроде px=0.008 не «прилипают» к нулю
			var e := SpinBox.new(); e.step=0.001; e.min_value=-1e5; e.max_value=1e5; e.value=float(val) if val!=null else 0.0
			e.value_changed.connect(func(v): rec[key]=snappedf(v, 0.001); if rec_idx>=0: _mark(rec_idx)); return e
		"int":
			var e := SpinBox.new(); e.step=1; e.min_value=-1000000; e.max_value=1000000; e.value=int(val) if val!=null else 0
			e.value_changed.connect(func(v): rec[key]=int(v); if rec_idx>=0: _mark(rec_idx)); return e
		"bool":
			var e := CheckBox.new(); e.button_pressed = bool(val) if val!=null else false
			e.toggled.connect(func(on): rec[key]=on; if rec_idx>=0: _mark(rec_idx)); return e
		"vec2", "vec3":
			var dims := 2 if t == "vec2" else 3
			var box := HBoxContainer.new()
			var arr: Array = val if typeof(val)==TYPE_ARRAY and (val as Array).size()==dims else []
			for d in dims:
				var sp := SpinBox.new()
				# мелкий шаг + snappedf на записи: без сеточного «прилипания»
				# и мусорных хвостов вида -17.3000000000029
				sp.step = 0.001; sp.min_value = -1e5; sp.max_value = 1e5
				sp.value = float(arr[d]) if arr.size()==dims else 0.0
				sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
				box.add_child(sp)
				sp.value_changed.connect(func(v):
					# менять только свою компоненту — чужие не перетирать
					var cur = rec.get(key)
					var out: Array = cur.duplicate() \
						if typeof(cur)==TYPE_ARRAY and (cur as Array).size()==dims \
						else ([0.0, 0.0] if dims==2 else [0.0, 0.0, 0.0])
					out[d] = snappedf(v, 0.001)
					rec[key] = out
					if rec_idx>=0: _mark(rec_idx))
			return box
		"color":
			# [r, g, b] (0..1) через палитру
			var e := ColorPickerButton.new(); e.edit_alpha = false
			e.custom_minimum_size = Vector2(110, 0)
			if typeof(val)==TYPE_ARRAY and (val as Array).size() >= 3:
				e.color = Color(float(val[0]), float(val[1]), float(val[2]))
			e.color_changed.connect(func(col: Color):
				rec[key] = [col.r, col.g, col.b]
				if rec_idx>=0: _mark(rec_idx))
			return e
		"enum", "dyn_enum":
			var e := OptionButton.new()
			var values: Array = []
			if field.get("nullable", false): values.append("")
			if t == "enum": values.append_array(field["options"])
			else: values.append_array(_dyn_options(str(field.get("source", ""))))
			var cur_s := str(val) if val != null else ""
			if not cur_s.is_empty() and not values.has(cur_s):
				values.append(cur_s)   # не терять значение, которого нет в источнике
			for v in values: e.add_item("(нет)" if str(v).is_empty() else str(v))
			var cur := values.find(cur_s)
			if cur >= 0: e.select(cur)
			e.item_selected.connect(func(i):
				rec[key] = null if str(values[i]).is_empty() else values[i]
				if rec_idx>=0: _mark(rec_idx))
			return e
		_:
			var e := TextEdit.new(); e.custom_minimum_size.y = 48; e.text = JSON.stringify(val) if val!=null else "null"
			e.text_changed.connect(func():
				var p = JSON.parse_string(e.text)
				var valid := (p != null) or (e.text.strip_edges() == "null")
				if valid:
					rec[key] = p
					if rec_idx >= 0:
						_mark(rec_idx)
				else:
					_set_st("%s: bad JSON" % key, false))
			return e

func _mark(idx: int) -> void:
	dirty = true
	_touch(_schema_file())
	var recs := _records()
	var row := rec_map.find(idx)
	if row >= 0 and row < rec_list.item_count and idx < recs.size():
		rec_list.set_item_text(row, _rec_title(recs[idx]))

# ──────────────────── ДАННЫЕ: динамические списки ─────────────────────────────

## id всех записей массива (сам массив или словарь с ключом root_key).
func _ids_of(data, root_key: String) -> Array:
	var arr = data
	if not root_key.is_empty() and typeof(data)==TYPE_DICTIONARY:
		arr = data.get(root_key, [])
	var out: Array = []
	if typeof(arr)==TYPE_ARRAY:
		for r in arr:
			if typeof(r)==TYPE_DICTIONARY and r.has("id"): out.append(str(r["id"]))
	return out

## Варианты dyn_enum-полей. Читает через file_cache — несохранённые правки
## соседних категорий сразу видны в выпадающих списках.
func _dyn_options(source: String) -> Array:
	match source:
		"npcs":    return _ids_of(_cached_file("npcs.json", []), "")
		"enemies": return _ids_of(_cached_file("enemies.json", {}), "enemies")
		"items":   return _ids_of(_cached_file("items.json", {}), "items")
		"targets":   # цели квестов: враги (kill) + предметы (collect)
			var out := _dyn_options("enemies")
			out.append_array(_dyn_options("items"))
			out.append("heart_1up")
			return out
		"quests":  return _ids_of(_cached_file("quests.json", []), "")
		"weapons": return _ids_of(_cached_file("weapons.json", []), "")
		"loot_items":   # предметы + спец-пикапы для таблиц лута
			var out := _dyn_options("items")
			out.append("heart_1up")
			return out
		"scenes":    # "story" (динамика story.rs) + сцены dialogues.json
			var out: Array = ["story"]
			out.append_array(_ids_of(_cached_file("dialogues.json", []), ""))
			return out
		"enemy_sprites":   # скан листов enemy_*.png — сгенерённые сразу в списке
			var out: Array = []
			var dir := DirAccess.open("res://assets/sprites/characters")
			if dir:
				for f in dir.get_files():
					if f.begins_with("enemy_") and f.ends_with(".png"):
						out.append(f.trim_prefix("enemy_").trim_suffix(".png"))
			out.sort()
			return out
	return []

# ──────────────────── ДАННЫЕ: CRUD ────────────────────────────────────────────

func _single_guard() -> bool:
	if _schema().get("single", false):
		_set_st("«%s» — одиночная запись, CRUD не применим" % category, false)
		return true
	return false

func _on_add() -> void:
	if _single_guard(): return
	var recs := _records(); var blank := {}
	for f in _schema()["fields"]:
		match f["type"]:
			"str","text": blank[f["key"]] = ""
			"float":      blank[f["key"]] = 0.0
			"int":        blank[f["key"]] = 0
			"bool":       blank[f["key"]] = false
			"enum":       blank[f["key"]] = null if f.get("nullable", false) else f["options"][0]
			"dyn_enum":
				var opts := _dyn_options(str(f.get("source", "")))
				blank[f["key"]] = null if f.get("nullable", false) \
					else (opts[0] if opts.size() > 0 else "")
			"vec2":  blank[f["key"]] = [0.0, 0.0]
			"vec3":  blank[f["key"]] = [0.0, 0.0, 0.0]
			"color": blank[f["key"]] = [1.0, 1.0, 1.0]
			_:
				# json: схема может задать дефолт (напр. [] у lines/choices —
				# null у них ломает serde и валидатор); копия, не общая ссылка
				var dv = f.get("default")
				blank[f["key"]] = dv.duplicate(true) \
					if typeof(dv) == TYPE_ARRAY or typeof(dv) == TYPE_DICTIONARY else dv
	# id-«уникализатор» только для свободных id; dyn_enum-id (ссылки на
	# существующие сущности, напр. лут) не затирать значением "new_N"
	if blank.has("id"):
		var id_is_ref := false
		for f2 in _schema()["fields"]:
			if f2["key"] == "id" and f2["type"] == "dyn_enum":
				id_is_ref = true
				break
		if not id_is_ref:
			blank["id"] = "new_%d" % (recs.size()+1)
	recs.append(blank); dirty = true; _touch(_schema_file())
	search_edit.text = ""   # новая запись не должна прятаться за фильтром
	_refresh_recs(recs.size()-1)

func _on_dup() -> void:
	if _single_guard(): return
	var idx := _sel_idx(); var recs := _records()
	if idx<0 or idx>=recs.size(): return
	var copy = recs[idx].duplicate(true)
	if typeof(copy)==TYPE_DICTIONARY and copy.has("id"): copy["id"] = str(copy["id"])+"_copy"
	recs.insert(idx+1, copy); dirty = true; _touch(_schema_file()); _refresh_recs(idx+1)

func _on_del() -> void:
	if _single_guard(): return
	var idx := _sel_idx(); var recs := _records()
	if idx<0 or idx>=recs.size(): return
	recs.remove_at(idx); dirty = true; _touch(_schema_file()); _refresh_recs(idx)

# ──────────────────── СОХРАНЕНИЕ ──────────────────────────────────────────────

func _save_all() -> void:
	var n := 0
	for rel in file_cache.keys():
		if synthetic.has(rel):
			continue   # файл возник в кэше как дефолт (для списков/валидации) — не создавать
		if _write_json("%s/%s" % [_preset_root(), rel], file_cache[rel]): n += 1
	dirty = false
	var warnings := _validate_preset()
	if warnings.is_empty():
		_set_st("Сохранено: %d файлов" % n)
	else:
		for w in warnings: print("[oh_editor] ⚠ %s" % w)
		_set_st("Сохранено: %d; ⚠ битых ссылок: %d — список в Output. Первая: %s"
			% [n, warnings.size(), warnings[0]], false)
	EditorInterface.get_resource_filesystem().scan()

## Проверка ссылочной целостности пресета (предупреждает, не блокирует;
## зеркалит cargo-тест content.rs::preset_tests).
func _validate_preset() -> Array:
	var warnings: Array = []
	var npc_ids   := _dyn_options("npcs")
	var enemy_ids := _dyn_options("enemies")
	var item_ids  := _dyn_options("items")
	var quest_ids := _dyn_options("quests")
	var scene_ids := _ids_of(_cached_file("dialogues.json", []), "")

	var quests = _cached_file("quests.json", [])
	if typeof(quests)==TYPE_ARRAY:
		for q in quests:
			if typeof(q)!=TYPE_DICTIONARY: continue
			var qid := str(q.get("id","?"))
			if not npc_ids.has(str(q.get("giver",""))):
				warnings.append("квест '%s': гивер '%s' не найден в NPC" % [qid, q.get("giver","")])
			var target := str(q.get("target",""))
			match str(q.get("kind","")):
				"kill":
					if not enemy_ids.has(target):
						warnings.append("квест '%s': цель kill '%s' не найдена во врагах" % [qid, target])
				"collect":
					if not (item_ids.has(target) or target=="heart_1up"):
						warnings.append("квест '%s': цель collect '%s' не найдена в предметах" % [qid, target])

	var dcfg = _cached_file("dungeon.json", {})
	if typeof(dcfg)==TYPE_DICTIONARY:
		var pools = dcfg.get("pools", [])
		if typeof(pools)==TYPE_ARRAY:
			for p in pools:
				if typeof(p)!=TYPE_DICTIONARY: continue
				var ens = p.get("enemies", [])
				if typeof(ens)!=TYPE_ARRAY: continue
				for en in ens:
					if not enemy_ids.has(str(en)):
						warnings.append("dungeon.json: в пуле враг '%s' не найден" % en)
		var ds = dcfg.get("settings", {})
		if typeof(ds)==TYPE_DICTIONARY:
			var boss := str(ds.get("boss", ""))
			if not boss.is_empty() and not enemy_ids.has(boss):
				warnings.append("dungeon.json: босс '%s' не найден во врагах" % boss)

	var lcfg = _cached_file("loot.json", {})
	if typeof(lcfg)==TYPE_DICTIONARY:
		var ri = lcfg.get("room_items", [])
		if typeof(ri)==TYPE_ARRAY:
			for e in ri:
				if typeof(e)==TYPE_DICTIONARY:
					var iid := str(e.get("id", ""))
					if not (item_ids.has(iid) or iid=="heart_1up"):
						warnings.append("loot.json: room_items '%s' не найден в предметах" % iid)
		var kd = lcfg.get("kill_drops", [])
		if typeof(kd)==TYPE_ARRAY:
			for e in kd:
				if typeof(e)==TYPE_DICTIONARY and str(e.get("kind",""))=="item":
					var did = e.get("id")
					if did != null and not (item_ids.has(str(did)) or str(did)=="heart_1up"):
						warnings.append("loot.json: kill_drops '%s' не найден в предметах" % did)

	var npcs = _cached_file("npcs.json", [])
	if typeof(npcs)==TYPE_ARRAY:
		for nrec in npcs:
			if typeof(nrec)!=TYPE_DICTIONARY: continue
			var nq = nrec.get("quest")
			if nq != null and not str(nq).is_empty() and not quest_ids.has(str(nq)):
				warnings.append("NPC '%s': квест '%s' не найден" % [nrec.get("id","?"), nq])
			var ns = nrec.get("scene")
			if ns != null and not str(ns).is_empty() and str(ns) != "story" \
					and not scene_ids.has(str(ns)):
				warnings.append("NPC '%s': сцены '%s' нет в dialogues.json (ок, если story-сцена)"
					% [nrec.get("id","?"), ns])

	for rel in file_cache.keys():
		if not str(rel).begins_with("maps/"): continue
		var m = file_cache[rel]
		if typeof(m)!=TYPE_DICTIONARY: continue
		var spawns = m.get("spawns", {})
		if typeof(spawns)!=TYPE_DICTIONARY: continue
		var sp_e = spawns.get("spawn_enemies", [])
		if typeof(sp_e)==TYPE_ARRAY:
			for s in sp_e:
				if typeof(s)==TYPE_DICTIONARY and not enemy_ids.has(str(s.get("kind",""))):
					warnings.append("%s: спавн врага '%s' — нет во врагах" % [rel, s.get("kind","")])
		var sp_i = spawns.get("spawn_items", [])
		if typeof(sp_i)==TYPE_ARRAY:
			for s in sp_i:
				if typeof(s)==TYPE_DICTIONARY:
					var k := str(s.get("kind",""))
					if not (item_ids.has(k) or k=="heart_1up"):
						warnings.append("%s: спавн предмета '%s' — нет в предметах" % [rel, k])

	var scenes = _cached_file("dialogues.json", [])
	if typeof(scenes)==TYPE_ARRAY:
		for sc in scenes:
			if typeof(sc)!=TYPE_DICTIONARY: continue
			var chs = sc.get("choices", [])
			if typeof(chs)!=TYPE_ARRAY: continue   # null/мусор — не валить валидатор
			for c in chs:
				if typeof(c)!=TYPE_DICTIONARY: continue
				var nx = c.get("next")
				if nx != null and not str(nx).is_empty() and not scene_ids.has(str(nx)):
					warnings.append("диалог '%s': next '%s' нет в dialogues.json (ок, если story-сцена)"
						% [sc.get("id","?"), nx])
	return warnings

# ──────────────────── ЗАМОК ЯДРА ──────────────────────────────────────────────

func _core_locked() -> bool:
	var probe := ProjectSettings.globalize_path(CORE_FILES[0]); var out := []
	OS.execute("attrib", [probe.replace("/","\\")], out)
	return out.size()>0 and "R" in str(out[0]).split(probe.replace("/","\\"))[0]

func _on_lock_tog(on: bool) -> void:
	var flag := "+R" if on else "-R"
	for f in CORE_FILES: OS.execute("attrib", [flag, ProjectSettings.globalize_path(f).replace("/","\\")])
	_refresh_lock(); _set_st("Ядро %s" % ("защищено" if on else "разблокировано"))

func _refresh_lock() -> void:
	var locked := _core_locked()
	lock_btn.set_pressed_no_signal(locked)
	lock_btn.text = "🔒 Ядро" if locked else "🔓 Ядро"

# ──────────────────── ВКЛАДКА «🎨 ИИ-ТЕКСТУРЫ» ─────────────────────────────────
# Генерация ассетов нейросетью «в несколько кликов»: тип + id + описание →
# tools/aigen.py (HTTP к серверу из tools/aigen.json + постобработка
# process_sprites.py). Итог сразу в godot/assets/*, игра подхватит при F5.

func _tools_dir() -> String:
	return ProjectSettings.globalize_path("res://").path_join("../tools").simplify_path()

func _build_gen_tab(parent: Control) -> void:
	var tpl = _read_json(_tools_dir().path_join("aigen_templates.json"))
	gen_templates = tpl if typeof(tpl) == TYPE_DICTIONARY else {}

	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	for side in ["margin_left", "margin_top", "margin_right", "margin_bottom"]:
		margin.add_theme_constant_override(side, 12)
	parent.add_child(margin)
	var v := VBoxContainer.new(); v.add_theme_constant_override("separation", 8)
	margin.add_child(v)

	var row_type := HBoxContainer.new(); row_type.add_child(_mk_lbl("Тип ассета:"))
	gen_type = OptionButton.new()
	for k in gen_templates.keys():
		if str(k).begins_with("_"): continue
		var idx := gen_type.item_count
		gen_type.add_item("%s — %s" % [k, gen_templates[k].get("title_ru", "")])
		gen_type.set_item_metadata(idx, k)
	gen_type.item_selected.connect(func(_i): _refresh_gen_tpl())
	row_type.add_child(gen_type)
	v.add_child(row_type)

	var row_id := HBoxContainer.new(); row_id.add_child(_mk_lbl("id:"))
	gen_id = LineEdit.new(); gen_id.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row_id.add_child(gen_id); v.add_child(row_id)

	v.add_child(_mk_lbl("Описание (вставляется в шаблон промпта):"))
	gen_desc = TextEdit.new(); gen_desc.custom_minimum_size.y = 72
	gen_desc.text_changed.connect(_refresh_gen_tpl)
	v.add_child(gen_desc)

	v.add_child(_mk_lbl("Итоговый промпт:"))
	gen_prompt_preview = Label.new()
	gen_prompt_preview.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	gen_prompt_preview.modulate = Color(0.7, 0.7, 0.8)
	v.add_child(gen_prompt_preview)

	gen_btn = Button.new(); gen_btn.text = "⚡ Сгенерировать"
	gen_btn.pressed.connect(_on_generate); v.add_child(gen_btn)

	gen_status = _mk_lbl("Сервер: tools/aigen.json (url/backend)")
	gen_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	v.add_child(gen_status)

	gen_preview = TextureRect.new()
	gen_preview.custom_minimum_size = Vector2(280, 280)
	gen_preview.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	gen_preview.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	gen_preview.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	gen_preview.size_flags_vertical = Control.SIZE_EXPAND_FILL
	v.add_child(gen_preview)
	_refresh_gen_tpl()

func _gen_sel_type() -> String:
	if gen_type == null or gen_type.selected < 0: return ""
	return str(gen_type.get_item_metadata(gen_type.selected))

func _refresh_gen_tpl() -> void:
	var t := _gen_sel_type()
	var tpl: Dictionary = gen_templates.get(t, {})
	gen_id.placeholder_text = str(tpl.get("id_hint", "id"))
	var d := gen_desc.text.strip_edges()
	gen_prompt_preview.text = str(tpl.get("prompt", "{desc}")) \
		.format({"desc": d if not d.is_empty() else "<описание>"})

func _on_generate() -> void:
	var type := _gen_sel_type()
	var id := gen_id.text.strip_edges()
	var desc := gen_desc.text.strip_edges().replace("\n", " ")
	if type.is_empty() or id.is_empty() or desc.is_empty():
		gen_status.text = "Заполни тип, id и описание"; return
	if gen_thread != null:
		gen_status.text = "Генерация уже идёт…"; return
	var py := "python"
	var cfg = _read_json(_tools_dir().path_join("aigen.json"))
	if typeof(cfg) == TYPE_DICTIONARY: py = str(cfg.get("python", "python"))
	gen_btn.disabled = true
	gen_status.text = "Генерация… (%s @ %s)" % [
		cfg.get("backend", "?") if typeof(cfg)==TYPE_DICTIONARY else "?",
		cfg.get("url", "?") if typeof(cfg)==TYPE_DICTIONARY else "?"]
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
		return  # вкладку уже уничтожили (плагин выключили во время генерации)
	gen_btn.disabled = false
	var text := ""
	for o in out: text += str(o)
	var ok_path := ""
	var err_msg := ""
	for line in text.split("\n"):
		var l: String = line.strip_edges()
		if l.begins_with("OK "): ok_path = l.substr(3)
		elif l.begins_with("ERR "): err_msg = l.substr(4)
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

# ──────────────────── УТИЛИТЫ ─────────────────────────────────────────────────

func _mk_lbl(t: String) -> Label: var l := Label.new(); l.text = t; return l

func _set_st(t: String, ok := true) -> void:
	if not status: return
	status.text = t; status.modulate = Color(0.6,1,0.7) if ok else Color(1,0.5,0.5)
