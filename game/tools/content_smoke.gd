extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	if not "--creative" in OS.get_cmdline_user_args():
		push_error("Run content_smoke.gd with -- --creative and an isolated APPDATA directory")
		quit(2)
		return
	# The caller uses an isolated APPDATA directory. A marker proves that creative
	# neither reads/renames an invalid campaign save nor replaces it on autosave.
	if OS.get_environment("APPDATA").contains("content-qa-user") and not FileAccess.file_exists("user://save.json"):
		var marker := FileAccess.open("user://save.json", FileAccess.WRITE)
		marker.store_string("creative-isolation-test-marker")
		marker.close()
	var save_before := FileAccess.get_file_as_string("user://save.json") if FileAccess.file_exists("user://save.json") else ""
	var game := (load("res://main.tscn") as PackedScene).instantiate()
	root.add_child(game)
	await process_frame
	var result: Dictionary = game.runtime_content_smoke()
	var failed := false
	for key in result:
		var ok: bool = result[key] >= 3 if key == "static_npcs" else (result[key] >= 10 if key == "decor" else (result[key] == 4 if key == "initial_styles" else bool(result[key])))
		print("CONTENT_SMOKE ", key, "=", result[key])
		if not ok:
			push_error("Content smoke failed: " + str(key))
			failed = true
	var save_after := FileAccess.get_file_as_string("user://save.json") if FileAccess.file_exists("user://save.json") else ""
	if save_before != save_after:
		failed = true
		push_error("Creative mode modified campaign save")
	game.runtime_smoke_shutdown_audio()
	game.queue_free()
	await process_frame
	var music := root.get_node_or_null("Music")
	if music != null:
		music.shutdown()
	await create_timer(0.25).timeout
	quit(1 if failed else 0)
