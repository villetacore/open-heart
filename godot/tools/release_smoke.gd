extends SceneTree


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var packed := load("res://main.tscn") as PackedScene
	if packed == null:
		push_error("RELEASE_SMOKE: main.tscn could not be loaded")
		quit(1)
		return

	var game := packed.instantiate()
	root.add_child(game)
	await process_frame
	await process_frame
	game.queue_free()
	await process_frame

	var music := root.get_node_or_null("/root/Music")
	if music == null:
		push_error("RELEASE_SMOKE: Music autoload is missing")
		quit(1)
		return
	music.shutdown()
	await create_timer(0.25).timeout
	for _frame in 3:
		await process_frame
	quit(0)
