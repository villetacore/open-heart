extends SceneTree
## Standalone art validation; does not instantiate gameplay or touch saves.

var failures: int = 0

func _initialize() -> void:
	call_deferred("_run")

func _texture(path: String, position: Vector2, size: Vector2) -> void:
	var texture := load(path) as Texture2D
	if texture == null:
		push_error("ART_REVIEW missing texture: " + path)
		failures += 1
		return
	if texture is AtlasTexture:
		var atlas := texture as AtlasTexture
		if not Rect2(Vector2.ZERO, atlas.atlas.get_size()).encloses(atlas.region):
			push_error("ART_REVIEW invalid region: " + path)
			failures += 1
	var image := TextureRect.new()
	image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	image.texture = texture
	image.position = position
	image.size = size
	image.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	root.add_child(image)

func _label(text: String, position: Vector2) -> void:
	var label := Label.new()
	label.text = text
	label.position = position
	label.add_theme_font_size_override("font_size", 18)
	root.add_child(label)

func _run() -> void:
	root.size = Vector2i(1280, 900)
	root.content_scale_size = Vector2i(1280, 900)
	var background := ColorRect.new()
	background.color = Color("100d18")
	background.size = Vector2(1280, 900)
	root.add_child(background)
	_label("OPENHEART / CHARACTER AND ICON REVIEW", Vector2(28, 12))
	var classes := ["berserk", "vanguard", "operator"]
	for index in classes.size():
		_texture("res://assets/portraits/classes/%s.tres" % classes[index], Vector2(28 + index * 260, 50), Vector2(240, 246))
	_texture("res://assets/illustrations/menu_femboy_city.png", Vector2(812, 50), Vector2(440, 246))
	_label("PERKS / 96 px and 48 px", Vector2(28, 310))
	var manifest: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://assets/icons/perks/manifest.json"))
	for index in manifest.entries.size():
		var entry: Dictionary = manifest.entries[index]
		var origin := Vector2(28 + (index % 9) * 137, 345 + (index / 9) * 160)
		_texture(entry.texture, origin, Vector2(96, 96))
		_texture(entry.texture, origin + Vector2(25, 100), Vector2(48, 48))
	_label("WEAPONS / 96 px", Vector2(28, 672))
	var weapons := ["sword", "chainsaw", "pistol", "shotgun", "rifle", "nailgun", "plasma", "rocket"]
	for index in weapons.size():
		_texture("res://assets/icons/weapons/%s.tres" % weapons[index], Vector2(28 + index * 150, 710), Vector2(96, 128))
	for frame in 5:
		await process_frame
	if DisplayServer.get_name() != "headless":
		await RenderingServer.frame_post_draw
		var output := ProjectSettings.globalize_path("res://../target/art-review.png")
		var error := root.get_texture().get_image().save_png(output)
		if error != OK:
			failures += 1
	print("ART_REVIEW checked=30 failures=", failures)
	var music := root.get_node_or_null("Music")
	if music != null and music.has_method("shutdown"):
		music.shutdown()
	await create_timer(0.2).timeout
	quit(1 if failures else 0)
