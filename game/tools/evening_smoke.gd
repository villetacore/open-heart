extends SceneTree
## Run with isolated APPDATA and -- --creative. Does not save the campaign.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	if not "--creative" in OS.get_cmdline_user_args() or not OS.get_environment("APPDATA").contains("evening-qa-user"):
		push_error("Use isolated evening-qa-user APPDATA and -- --creative")
		quit(2)
		return
	var before := FileAccess.get_file_as_string("user://save.json") if FileAccess.file_exists("user://save.json") else ""
	var game := (load("res://main.tscn") as PackedScene).instantiate()
	root.add_child(game)
	await process_frame
	var result: Dictionary = game.runtime_evening_smoke()
	var failed := result.is_empty()
	for key in result:
		print("EVENING_SMOKE ", key, "=", result[key])
		if not bool(result[key]):
			push_error("EVENING_SMOKE failed: " + str(key))
			failed = true
	var after := FileAccess.get_file_as_string("user://save.json") if FileAccess.file_exists("user://save.json") else ""
	if before != after:
		push_error("Campaign save was modified")
		failed = true
	if DisplayServer.get_name() != "headless":
		await process_frame
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png("res://../target/evening-review.png")
	game.runtime_smoke_shutdown_audio()
	game.queue_free()
	await process_frame
	var music := root.get_node_or_null("Music")
	if music != null:
		music.shutdown()
	await create_timer(0.25).timeout
	quit(1 if failed else 0)
