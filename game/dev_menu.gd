extends Node

# Временный инструмент: прокликивает экран сетевой игры и снимает кадры.
# Меню ловит мышь через _input, поэтому клики синтезируем настоящими событиями.

const CONNECT := Vector2(960, 716)     # кнопка «Подключиться» в дизайн-координатах 1920×1080
const FIRST_ROW := Vector2(760, 413)   # первая строка списка серверов
const HOST := Vector2(706, 807)        # кнопка «Создать игру с другом»

func _ready() -> void:
	var scene: PackedScene = load("res://main_menu.tscn")
	add_child(scene.instantiate())
	await get_tree().create_timer(1.5).timeout

	await _shot("menu")
	_click(CONNECT)
	await get_tree().create_timer(2.5).timeout
	await _shot("net")

	# Создать игру с другом: сервер поднимается дочерним процессом.
	_click(HOST)
	await get_tree().create_timer(16.0).timeout
	await _shot("host")

	get_tree().quit()

func _click(pos: Vector2) -> void:
	# Синтетическое событие живёт в пикселях окна, а меню считает в 1920×1080:
	# растяжка canvas_items приведёт координаты сама.
	var window := Vector2(DisplayServer.window_get_size())
	var scaled := Vector2(pos.x * window.x / 1920.0, pos.y * window.y / 1080.0)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = pressed
		event.position = scaled
		event.global_position = scaled
		Input.parse_input_event(event)
	await get_tree().process_frame

func _shot(name: String) -> void:
	for i in 4:
		await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	img.save_png("user://menu_%s.png" % name)
	print("SHOT ", name)
