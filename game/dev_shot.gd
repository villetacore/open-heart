extends Node

# Временный инструмент: загружает основную сцену и снимает несколько кадров
# карты с заданных точек. Нужен, чтобы смотреть на хаб глазами, а не по JSON.

const SHOTS := [
	{"name": "aerial", "pos": Vector3(0, 96, 96), "look": Vector3(0, 0, -10)},
	{"name": "plaza", "pos": Vector3(0, 3.0, 26), "look": Vector3(0, 3.0, -20)},
	{"name": "boulevard", "pos": Vector3(0, 3.0, -20), "look": Vector3(0, 4.0, -58)},
	{"name": "market", "pos": Vector3(-34, 3.0, 34), "look": Vector3(-34, 2.0, 10)},
	{"name": "skywalk", "pos": Vector3(-40, 12.0, 12), "look": Vector3(10, 2.0, -20)},
	{"name": "shrine", "pos": Vector3(0, 8.0, -76), "look": Vector3(0, 3.0, -50)},
]

func _ready() -> void:
	var scene: PackedScene = load("res://main.tscn")
	add_child(scene.instantiate())
	await get_tree().create_timer(4.0).timeout

	# Первый кадр — глазами игрока: так виден и сетевой режим, и HUD.
	await _shot("player")

	var cam := Camera3D.new()
	cam.fov = 70.0
	cam.far = 400.0
	add_child(cam)

	for shot in SHOTS:
		cam.global_position = shot["pos"]
		cam.look_at(shot["look"], Vector3.UP)
		cam.make_current()
		await _shot(shot["name"])
	get_tree().quit()

func _shot(name: String) -> void:
	for i in 6:
		await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	img.save_png("user://shot_%s.png" % name)
	print("SHOT ", name, " -> ", ProjectSettings.globalize_path("user://shot_%s.png" % name))
