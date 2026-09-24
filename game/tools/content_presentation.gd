extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _capture(page: String) -> void:
	for frame in 8:
		await process_frame
	await RenderingServer.frame_post_draw
	var output := ProjectSettings.globalize_path("res://../target/content-review")
	DirAccess.make_dir_recursive_absolute(output)
	root.get_texture().get_image().save_png(output.path_join(page + ".png"))

func _run() -> void:
	var menu := (load("res://main_menu.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	await _capture("menu")
	menu.queue_free()
	await process_frame
	var game := (load("res://main.tscn") as PackedScene).instantiate()
	root.add_child(game)
	await process_frame
	for page in ["world", "quarter", "atelier", "wardrobe", "perks", "creative"]:
		game.runtime_review_page(page)
		await _capture(page)
	game.runtime_smoke_shutdown_audio()
	game.queue_free()
	await process_frame
	var music := root.get_node_or_null("Music")
	if music != null:
		music.shutdown()
	await create_timer(0.2).timeout
	quit()
