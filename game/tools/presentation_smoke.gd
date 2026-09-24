extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var game := (load("res://main.tscn") as PackedScene).instantiate()
	root.add_child(game)
	await process_frame
	var output := ProjectSettings.globalize_path("res://../target/review")
	DirAccess.make_dir_recursive_absolute(output)
	for page in ["classes", "inventory", "perks", "journal", "world", "dungeon"]:
		game.runtime_review_page(page)
		for frame in 8:
			await process_frame
		await RenderingServer.frame_post_draw
		var result := root.get_texture().get_image().save_png(output.path_join(page + ".png"))
		if result != OK:
			push_error("Cannot save review frame: " + page)
			quit(1)
			return
		print("REVIEW_FRAME ", page)
	game.runtime_smoke_shutdown_audio()
	game.queue_free()
	await process_frame
	var music := root.get_node_or_null("Music")
	if music != null:
		music.shutdown()
	await create_timer(0.3).timeout
	quit(0)
