extends Node
## Фоновая музыка OpenHeart.
## Автозагрузка (singleton): крутит плейлист по кругу в случайном порядке,
## переживает смену сцен (меню ↔ игра). Логика игры на Rust её не трогает —
## достаточно, что узел висит автозагрузкой.

const TRACKS: Array[String] = [
	"res://assets/music/Chrome Coil.ogg",
	"res://assets/music/Chrome Pulse.ogg",
	"res://assets/music/Chrome Rave Loop.ogg",
	"res://assets/music/Neon Acid Rain.ogg",
	"res://assets/music/Neon Static.ogg",
	"res://assets/music/Static in the Rain.ogg",
]

## Громкость музыки в дБ (0 = исходная, отрицательные значения тише).
@export var volume_db: float = -8.0

var _player: AudioStreamPlayer
var _order: Array[int] = []
var _idx: int = 0
var _last_scene: String = ""

func _ready() -> void:
	# Музыка не должна замолкать, когда игра на паузе (меню, диалоги).
	process_mode = Node.PROCESS_MODE_ALWAYS

	_ensure_buses()

	_player = AudioStreamPlayer.new()
	_player.bus = &"Music"
	_player.volume_db = volume_db
	add_child(_player)
	_player.finished.connect(_on_finished)

	_reshuffle()
	_play_current()

## Завести аудио-шины Music и SFX (маршрут в Master), если их ещё нет —
## по ним рулят громкостью настройки (Settings.apply_audio в Rust).
func _ensure_buses() -> void:
	for bus_name in ["Music", "SFX"]:
		if AudioServer.get_bus_index(bus_name) == -1:
			var idx := AudioServer.bus_count
			AudioServer.add_bus(idx)
			AudioServer.set_bus_name(idx, bus_name)
			AudioServer.set_bus_send(idx, "Master")

func _process(_delta: float) -> void:
	# Меняем трек при смене сцены (меню -> игра -> меню), чтобы музыка
	# заметно менялась по ходу игры, а не только когда трек доиграл до конца.
	var scene := get_tree().current_scene
	if scene == null:
		return
	var name := String(scene.name)
	if _last_scene == "":
		_last_scene = name  # первичная инициализация — трек не трогаем
	elif name != _last_scene:
		_last_scene = name
		skip()

## Перемешать порядок треков (каждый круг — новый порядок).
func _reshuffle() -> void:
	_order.clear()
	for i in TRACKS.size():
		_order.append(i)
	_order.shuffle()
	_idx = 0

func _play_current() -> void:
	if _order.is_empty():
		return
	for _attempt in TRACKS.size():
		var path: String = TRACKS[_order[_idx]]
		# Читаем исходный OGG напрямую. Это не зависит от версии кэша
		# `.godot/imported/*.oggvorbisstr` и переживает перенос проекта между Godot.
		var stream := AudioStreamOggVorbis.load_from_file(path)
		if stream != null:
			stream.loop = false
			_player.stream = stream
			_player.play()
			return
		push_warning("Music: не удалось прочитать OGG " + path)
		_advance_index()
	push_warning("Music: ни один трек плейлиста не удалось загрузить")
	_player.stream = null

func _advance_index() -> void:
	_idx += 1
	if _idx >= _order.size():
		_reshuffle()

func _on_finished() -> void:
	_advance_index()
	_play_current()

## Публичный API — можно дёргать из других сцен/скриптов.

## Пропустить текущий трек.
func skip() -> void:
	_on_finished()

## Задать громкость (дБ).
func set_volume(db: float) -> void:
	volume_db = db
	if _player != null:
		_player.volume_db = db

## Включить/выключить музыку.
func set_enabled(enabled: bool) -> void:
	if _player == null:
		return
	if enabled and not _player.playing:
		_play_current()
	elif not enabled and _player.playing:
		_player.stop()

func shutdown() -> void:
	set_process(false)
	if _player == null:
		return
	_player.stop()
	_player.stream = null
	_player.free()
	_player = null

func _exit_tree() -> void:
	shutdown()
