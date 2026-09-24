extends "res://tools/art_review.gd"
## Cosmetic/decor art only. Does not load gameplay or modify player saves.

func _run() -> void:
	root.size = Vector2i(1280, 900)
	root.content_scale_size = Vector2i(1280, 900)
	var background := ColorRect.new()
	background.color = Color("100d18")
	background.size = Vector2(1280, 900)
	root.add_child(background)
	_label("OPENHEART / FEMBOY COSTUMES AND QUARTER", Vector2(28, 12))
	_texture("res://assets/illustrations/costumes/femboy_classes_fullbody.png", Vector2(28, 50), Vector2(660, 440))
	var decor: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://assets/decor/femboy_quarter/manifest.json"))
	for index in decor.entries.size():
		var entry: Dictionary = decor.entries[index]
		_texture(entry.texture, Vector2(712 + (index % 3) * 180, 52 + (index / 3) * 218), Vector2(166, 208))
		var packed := load(entry.scene) as PackedScene
		if packed == null:
			failures += 1
			continue
		var instance := packed.instantiate()
		var artwork := instance.get_node_or_null("Artwork") as Sprite3D
		if artwork == null or artwork.texture == null:
			failures += 1
		instance.free()
	_label("STYLE COLLECTION / 96 px and 48 px / artwork ready for integration", Vector2(28, 512))
	var cosmetics: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://assets/icons/cosmetics/manifest.json"))
	for index in cosmetics.entries.size():
		var entry: Dictionary = cosmetics.entries[index]
		var origin := Vector2(28 + (index % 6) * 204, 550 + (index / 6) * 164)
		_texture(entry.texture, origin, Vector2(96, 96))
		_texture(entry.texture, origin + Vector2(105, 24), Vector2(48, 48))
	for frame in 5:
		await process_frame
	if DisplayServer.get_name() != "headless":
		await RenderingServer.frame_post_draw
		var output := ProjectSettings.globalize_path("res://../target/art-wave02-review.png")
		if root.get_texture().get_image().save_png(output) != OK:
			failures += 1
	print("ART_WAVE02 textures=19 prefabs=6 failures=", failures)
	var music := root.get_node_or_null("Music")
	if music != null and music.has_method("shutdown"):
		music.shutdown()
	await create_timer(0.2).timeout
	quit(1 if failures else 0)
