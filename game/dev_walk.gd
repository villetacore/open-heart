extends Node

# Временный инструмент: проверяет, что до врат можно дойти ногами.
# Игрок ставится на проспект, поворачивается к вратам и идёт вперёд.

const START := Vector3(0, 2.0, 12)   # спавн игрока в хабе
const SECONDS := 14

func _ready() -> void:
	add_child(load("res://main.tscn").instantiate())
	await get_tree().create_timer(4.0).timeout

	var player := _find_player(get_tree().root)
	if player == null:
		print("WALK: игрок не найден")
		get_tree().quit()
		return
	player.global_position = START
	player.rotation.y = 0.0
	await get_tree().physics_frame

	Input.action_press("move_forward")
	for step in 22:
		await get_tree().create_timer(1.0).timeout
		var p: Vector3 = player.global_position
		print("WALK t=%d  x=%.1f y=%.2f z=%.1f" % [step + 1, p.x, p.y, p.z])
	Input.action_release("move_forward")
	await _shot("at_gate")

	# А теперь честно жмём E.
	Input.action_press("interact")
	await get_tree().create_timer(0.2).timeout
	Input.action_release("interact")
	await get_tree().create_timer(2.0).timeout
	var q: Vector3 = player.global_position
	print("AFTER-E  x=%.1f y=%.2f z=%.1f" % [q.x, q.y, q.z])
	await _shot("after_e")
	get_tree().quit()

func _shot(name: String) -> void:
	for i in 4:
		await RenderingServer.frame_post_draw
	get_viewport().get_texture().get_image().save_png("user://walk_%s.png" % name)
	print("SHOT ", name)

func _find_player(node: Node) -> Node3D:
	if node is CharacterBody3D and node.name.to_lower().contains("player"):
		return node
	for child in node.get_children():
		var found := _find_player(child)
		if found != null:
			return found
	return null
