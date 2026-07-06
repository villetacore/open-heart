@tool
extends EditorPlugin
## OpenHeart Editor — главная панель редактора игры (рядом с 2D/3D/Script).
## Философия: этот плагин — ЕДИНСТВЕННОЕ место, где контент-мейкер меняет игру.
## Он редактирует только data-файлы пресетов (presets/<id>/*.json) и умеет
## блокировать фундаментальные файлы игры от случайной правки.

const MainPanel := preload("res://addons/oh_editor/editor_main.gd")

var panel: Control


func _enter_tree() -> void:
	panel = MainPanel.new()
	get_editor_interface().get_editor_main_screen().add_child(panel)
	_make_visible(false)


func _exit_tree() -> void:
	if panel:
		panel.queue_free()


func _has_main_screen() -> bool:
	return true


func _make_visible(visible: bool) -> void:
	if panel:
		panel.visible = visible


func _get_plugin_name() -> String:
	return "OpenHeart"


func _get_plugin_icon() -> Texture2D:
	var icon := load("res://assets/ui/ui_heart.png")
	if icon is Texture2D:
		var img: Image = icon.get_image()
		img.resize(16, 16, Image.INTERPOLATE_NEAREST)
		return ImageTexture.create_from_image(img)
	return get_editor_interface().get_base_control().get_theme_icon("Heart", "EditorIcons")
