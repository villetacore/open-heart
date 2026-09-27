extends SceneTree

var failed := false


func _initialize() -> void:
	call_deferred("_run")


func _check(condition: bool, message: String) -> void:
	if condition:
		return
	failed = true
	push_error("RUNTIME_SMOKE: " + message)


func _run() -> void:
	var menu_packed := load("res://main_menu.tscn") as PackedScene
	_check(menu_packed != null, "main_menu.tscn could not be loaded")
	if menu_packed != null:
		var menu := menu_packed.instantiate()
		root.add_child(menu)
		await process_frame
		var menu_locale: Dictionary = menu.runtime_smoke_localization()
		_check(menu_locale.ru and menu_locale.en, "main menu labels did not switch RU/EN live")
		_check(menu_locale.changed, "main menu subtitle did not change language")
		_check(menu_locale.settings, "settings labels did not switch to English")
		print("RUNTIME_SMOKE menu_locale ru=%s en=%s settings=%s" % [menu_locale.ru, menu_locale.en, menu_locale.settings])
		menu.queue_free()
		await process_frame

	var packed := load("res://main.tscn") as PackedScene
	_check(packed != null, "main.tscn could not be loaded")
	if packed == null:
		quit(1)
		return

	var game := packed.instantiate()
	root.add_child(game)
	await process_frame
	var districts: Dictionary = game.runtime_smoke_world_districts()
	_check(districts.total >= 6, "world has fewer than six districts")
	_check(districts.recognized == districts.total, "not every district is recognized from player position")
	_check(districts.decor_clusters >= districts.total, "district decoration clusters are incomplete")
	_check(districts.decor_nodes >= 100, "hub decoration density is unexpectedly low")
	_check(districts.route_layers >= 8, "hub readable route layer is incomplete")
	_check(districts.route_nodes >= 50, "hub route geometry is unexpectedly sparse")
	_check(districts.skyline_beacons >= 10, "hub skyline lacks secondary landmarks")
	_check(districts.beacon_nodes >= districts.skyline_beacons * 4, "hub skyline beacon construction is incomplete")
	_check(districts.ambient == districts.total, "not every district has animated ambience")
	_check(districts.ambient_moved == districts.ambient, "ambient anchors did not move")
	_check(districts.ambient_profiles == districts.ambient, "district ambient profiles are not distinct")
	_check(districts.ambient_paused, "ambient animation ignored pause mode")
	print("RUNTIME_SMOKE districts total=%s recognized=%s clusters=%s decor=%s routes=%s/%s beacons=%s ambient=%s" % [districts.total, districts.recognized, districts.decor_clusters, districts.decor_nodes, districts.route_layers, districts.route_nodes, districts.skyline_beacons, districts.ambient])
	var enemy_roster: Dictionary = game.runtime_smoke_enemy_roster()
	_check(enemy_roster.total >= 20, "enemy roster is unexpectedly small")
	_check(enemy_roster.roles == 7, "enemy roster does not cover seven tactical roles")
	print("RUNTIME_SMOKE enemies total=%s roles=%s" % [enemy_roster.total, enemy_roster.roles])
	var quest_chains: Dictionary = game.runtime_smoke_complete_quest_chains()
	_check(quest_chains.total >= 20, "quest roster is unexpectedly small")
	_check(not quest_chains.stalled, "quest dependency graph stalled at runtime")
	_check(quest_chains.offered == quest_chains.total, "not every quest can be accepted")
	_check(quest_chains.completed == quest_chains.total, "not every quest can be completed")
	_check(quest_chains.exploration_completed == 2, "dungeon exploration quest chain is incomplete")
	_check(quest_chains.chains >= 8, "quest roster is missing the new quarter chains")
	_check(quest_chains.reward_quests == quest_chains.total, "not every quest has an item reward")
	_check(quest_chains.awarded_reward_qty == quest_chains.expected_reward_qty, "quest item rewards were not fully granted")
	_check(quest_chains.journal_localized, "quest chain journal is not localized")
	print("RUNTIME_SMOKE quests total=%s offered=%s completed=%s" % [
		quest_chains.total,
		quest_chains.offered,
		quest_chains.completed,
	])
	var world_changes: Dictionary = game.runtime_smoke_quest_world_changes()
	_check(world_changes.configured >= 5, "quest finales do not configure at least five world changes")
	_check(world_changes.built == world_changes.configured, "completed quest chains did not transform every district")
	_check(world_changes.nodes >= world_changes.built * 6, "quest world transformations are visually incomplete")
	_check(world_changes.patterns == world_changes.configured, "quest world transformations are not visually distinct")
	_check(world_changes.activity_profiles >= 5, "quest finales use fewer than five distinct activity profiles")
	_check(world_changes.activities >= world_changes.expected_activities - 4, "quest finale activities were not spawned")
	_check(world_changes.activities >= 20, "restored districts are still too empty")
	_check(world_changes.activity_moved, "quest finale activities are not animated")
	_check(world_changes.activity_paused, "quest finale activities ignored pause mode")
	_check(world_changes.persisted, "quest world transformations do not survive save round-trip")
	print("RUNTIME_SMOKE world_changes built=%s nodes=%s patterns=%s activities=%s/%s persisted=%s" % [world_changes.built, world_changes.nodes, world_changes.patterns, world_changes.activities, world_changes.activity_profiles, world_changes.persisted])
	var quest_navigation: Dictionary = game.runtime_smoke_quest_navigation()
	_check(quest_navigation.localized, "tracked quest objective is not localized RU/EN")
	_check(quest_navigation.described, "tracked quest objective lacks a readable description")
	_check(quest_navigation.distance, "tracked quest objective lacks distance guidance")
	_check(quest_navigation.target, "active world quest did not resolve a navigation target")
	_check(quest_navigation.marker_visible, "active quest world marker is hidden")
	_check(quest_navigation.cycled, "tracked quest cannot cycle between active objectives")
	print("RUNTIME_SMOKE quest_navigation localized=%s target=%s cycle=%s" % [quest_navigation.localized, quest_navigation.target, quest_navigation.cycled])
	var quest_journal: Dictionary = game.runtime_smoke_quest_journal()
	_check(quest_journal.localized, "quest journal is not localized RU/EN")
	_check(quest_journal.chains >= 8, "quest journal is missing the new quarter chains")
	_check(quest_journal.states, "quest journal does not distinguish active and completed stages")
	_check(quest_journal.rewards, "quest journal omits stage rewards")
	_check(quest_journal.panel_visible, "quest journal panel did not open")
	_check(quest_journal.tracked, "quest journal selection did not change tracked objective")
	_check(quest_journal.closed, "quest journal did not restore explore mode")
	print("RUNTIME_SMOKE quest_journal chains=%s localized=%s tracked=%s" % [quest_journal.chains, quest_journal.localized, quest_journal.tracked])
	var weapons: Dictionary = game.runtime_smoke_weapon_profiles()
	_check(weapons.total == 8, "weapon roster does not contain eight slots")
	_check(weapons.animated == weapons.total, "not every weapon completes switch animation")
	_check(weapons.feedback_complete == weapons.total, "weapon feedback profile is incomplete")
	_check(weapons.feedback_profiles == weapons.total, "weapon feedback profiles are not unique")
	_check(weapons.audio_complete == weapons.total, "weapon audio profile is incomplete")
	_check(weapons.audio_profiles == weapons.total, "weapon audio profiles are not unique")
	_check(weapons.audio_assets_loaded == weapons.audio_assets, "weapon audio asset failed to load")
	_check(weapons.impact_audio_players == weapons.total, "weapon impact audio players were not created")
	_check(weapons.fire_audio_played, "weapon fire audio player was not created")
	_check(weapons.accuracy_complete == weapons.total, "weapon accuracy profile is incomplete")
	_check(weapons.accuracy_profiles == weapons.total, "weapon accuracy profiles are not unique")
	_check(weapons.bloom_peak > 0.0, "weapon bloom did not increase")
	_check(weapons.bloom_recovered, "weapon bloom did not recover")
	_check(weapons.crosshair_expanded, "crosshair did not reflect weapon bloom")
	_check(weapons.hit_confirmed, "crosshair did not confirm a hit")
	_check(weapons.kill_confirmed, "crosshair did not confirm a kill")
	_check(weapons.alt_profiles == weapons.total, "weapon alt-fire profiles are incomplete or duplicated")
	_check(weapons.alt_melee == 2, "alt-fire roster does not contain two melee modes")
	_check(weapons.alt_hitscan == 4, "alt-fire roster does not contain four hitscan modes")
	_check(weapons.alt_projectile == 2, "alt-fire roster does not contain two projectile modes")
	_check(weapons.alt_animation_complete == weapons.total, "alt-fire animation data is incomplete")
	_check(weapons.alt_animation_distinct == weapons.total, "alt-fire reuses a primary animation clip")
	_check(weapons.alt_animation_profiles == weapons.total, "alt-fire animation profiles are duplicated")
	_check(weapons.alt_animation_started, "alt-fire did not enter its secondary animation state")
	_check(weapons.alt_spawned_projectile, "rocket alt-fire did not spawn a projectile")
	_check(weapons.alt_extended_cooldown, "alt-fire cooldown multiplier was ignored")
	_check(weapons.alt_added_bloom, "alt-fire bloom multiplier was ignored")
	_check(weapons.impact_fx == weapons.total, "not every weapon spawned an impact effect")
	_check(weapons.impact_lights == weapons.total, "not every weapon spawned impact lighting")
	_check(weapons.tracer_fx >= weapons.total * 2, "weapon tracer trails were not spawned")
	_check(weapons.muzzle_active, "weapon muzzle feedback did not activate")
	_check(weapons.ammo_consumed, "weapon magazine did not consume a shot")
	_check(weapons.recoil_applied, "weapon recoil did not affect aim")
	print("RUNTIME_SMOKE weapons total=%s animated=%s feedback=%s audio=%s accuracy=%s alt=%s tracer_fx=%s recoil=%s" % [
		weapons.total,
		weapons.animated,
		weapons.feedback_profiles,
		weapons.audio_profiles,
		weapons.accuracy_profiles,
		weapons.alt_profiles,
		weapons.tracer_fx,
		weapons.recoil_applied,
	])
	var weapon_mods: Dictionary = game.runtime_smoke_weapon_mods()
	_check(weapon_mods.total == 16, "weapon mod roster does not contain sixteen branches")
	_check(weapon_mods.localized == weapon_mods.total, "weapon mods are not fully localized")
	_check(weapon_mods.visual_complete == weapon_mods.total, "weapon mod feedback profile is incomplete")
	_check(weapon_mods.visual_profiles == weapon_mods.total, "weapon mod visual profiles are not unique")
	_check(weapon_mods.effective_profiles == weapon_mods.total, "weapon mod profiles were not applied to runtime feedback")
	_check(weapon_mods.branch_pairs_distinct == 8, "weapon mod branch feedback is not distinct for every weapon")
	_check(weapon_mods.branch_impact_fx == weapon_mods.total, "weapon mod impact effects were not spawned")
	_check(weapon_mods.branch_impact_lights == weapon_mods.total, "weapon mod impact lights were not spawned")
	_check(weapon_mods.first_selected == 1 and weapon_mods.first_cores == 1, "first weapon mod branch was not purchased")
	_check(weapon_mods.second_selected == 2 and weapon_mods.second_cores == 0, "weapon mod branch did not switch exclusively")
	_check(weapon_mods.first_changes_stats, "weapon mod does not alter combat stats")
	_check(weapon_mods.view_identity_distinct, "weapon mod branches have the same FP weapon identity")
	_check(weapon_mods.view_visual_applied, "selected weapon mod was not applied to the FP weapon")
	print("RUNTIME_SMOKE weapon_mods total=%s localized=%s profiles=%s effective=%s pairs=%s branches=%s/%s" % [
		weapon_mods.total,
		weapon_mods.localized,
		weapon_mods.visual_profiles,
		weapon_mods.effective_profiles,
		weapon_mods.branch_pairs_distinct,
		weapon_mods.first_selected,
		weapon_mods.second_selected,
	])
	var weak_points: Dictionary = game.runtime_smoke_weak_points()
	_check(weak_points.configured >= 20, "enemy weak-point profiles are incomplete")
	_check(weak_points.boss_profiles == 4, "bosses lack custom weak-point profiles")
	_check(weak_points.critical, "upper-body hit was not classified as critical")
	_check(weak_points.critical_damage > weak_points.normal_damage, "critical hit did not increase damage")
	_check(weak_points.critical_crosshair, "critical hit has no crosshair confirmation")
	_check(weak_points.localized, "weak-point HUD labels are not localized")
	print("RUNTIME_SMOKE weak_points configured=%s bosses=%s damage=%.2f/%.2f" % [
		weak_points.configured,
		weak_points.boss_profiles,
		weak_points.normal_damage,
		weak_points.critical_damage,
	])
	var dialogue_conditions: Dictionary = game.runtime_smoke_dialogue_conditions()
	_check(dialogue_conditions.reactive == 4, "boss aftermath does not select four reactive dialogue scenes")
	_check(dialogue_conditions.localized == 4, "reactive dialogue scenes are not fully localized")
	_check(dialogue_conditions.low_stat_choices == 1, "low-stat dialogue choice was not gated")
	_check(dialogue_conditions.high_stat_choices == 2, "high-stat dialogue choice did not unlock")
	_check(dialogue_conditions.not_flag_before and not dialogue_conditions.not_flag_after, "not_flag dialogue condition failed")
	_check(dialogue_conditions.quest_conditions >= 1, "quest-completion dialogue condition is missing")
	print("RUNTIME_SMOKE dialogues reactive=%s localized=%s choices=%s/%s" % [
		dialogue_conditions.reactive,
		dialogue_conditions.localized,
		dialogue_conditions.low_stat_choices,
		dialogue_conditions.high_stat_choices,
	])
	var hazard_profiles: Dictionary = game.runtime_smoke_hazard_profiles()
	_check(hazard_profiles.types == 4, "hazard roster does not contain four types")
	_check(hazard_profiles.active_profiles == 4, "not every hazard has an active phase")
	_check(hazard_profiles.warning_profiles == 3, "pulsed hazards lack warning phases")
	_check(hazard_profiles.dormant_profiles == 3, "pulsed hazards lack dormant phases")
	_check(hazard_profiles.statuses == 4, "hazards do not use four distinct statuses")
	_check(hazard_profiles.localized == 4, "hazard warnings are not fully localized")
	print("RUNTIME_SMOKE hazards types=%s phased=%s statuses=%s" % [
		hazard_profiles.types,
		hazard_profiles.warning_profiles,
		hazard_profiles.statuses,
	])
	var quest_events: Dictionary = game.runtime_smoke_prepare_quest_events()
	_check(quest_events.stranger_found, "quest interaction target is missing")
	_check(quest_events.interact_progress == 1, "interact quest event failed")

	var expected_bosses := ["crypt_warden", "blood_oracle", "archive_sentinel", "heart_tyrant"]
	var expected_boss_roles := ["tank", "support", "controller", "commander"]
	for depth in [1, 2, 3, 4]:
		var dungeon: Dictionary = game.runtime_smoke_enter_dungeon(depth)
		_check(dungeon.depth == depth, "wrong depth %s" % depth)
		_check(dungeon.root, "missing dungeon root at depth %s" % depth)
		_check(dungeon.navigation, "missing navigation at depth %s" % depth)
		_check(dungeon.floor_cells > 0, "empty floor map at depth %s" % depth)
		_check(dungeon.room_count >= 8, "too few authored rooms at depth %s" % depth)
		_check(dungeon.map_rooms == dungeon.room_count, "not every room is mapped at depth %s" % depth)
		_check(dungeon.map_hazards == dungeon.hazards, "not every hazard is mapped at depth %s" % depth)
		_check(dungeon.dressing_nodes >= dungeon.room_count * 4, "room dressing density is too low at depth %s" % depth)
		_check(dungeon.room_focals == dungeon.room_count, "not every room has a focal setpiece at depth %s" % depth)
		_check(dungeon.focal_nodes >= dungeon.room_count * 4, "room focal composition is incomplete at depth %s" % depth)
		_check(dungeon.focal_profiles >= 4, "room focal profiles are not varied at depth %s" % depth)
		_check(dungeon.dressing_variants == 3, "room dressing lacks three seeded variants at depth %s" % depth)
		_check(dungeon.map_ready, "semantic minimap texture is missing at depth %s" % depth)
		_check(dungeon.map_legend, "minimap legend is hidden at depth %s" % depth)
		_check(dungeon.has_safe, "safe room is missing at depth %s" % depth)
		_check(dungeon.has_treasure, "treasure room is missing at depth %s" % depth)
		_check(dungeon.has_ritual, "ritual room is missing at depth %s" % depth)
		_check(dungeon.has_antechamber, "boss antechamber is missing at depth %s" % depth)
		if depth >= 2:
			_check(dungeon.has_traversal, "traversal room is missing at depth %s" % depth)
			_check(dungeon.has_puzzle, "puzzle room is missing at depth %s" % depth)
			_check(dungeon.puzzle_switches >= 3, "puzzle room lacks three relays at depth %s" % depth)
		if depth >= 3:
			_check(dungeon.has_story, "story room is missing at depth %s" % depth)
			_check(dungeon.story_echoes >= 1, "story room lacks a memory echo at depth %s" % depth)
		_check(dungeon.enemies > 0, "no enemies at depth %s" % depth)
		_check(dungeon.exit_portal, "missing exit portal at depth %s" % depth)
		_check(dungeon.next_portal, "missing next portal at depth %s" % depth)
		_check(dungeon.portal_reachable, "next portal is unreachable at depth %s" % depth)
		if depth == 1:
			var enemy_animations: Dictionary = game.runtime_smoke_enemy_animations()
			_check(enemy_animations.total > 0, "enemy animation smoke found no actors")
			_check(enemy_animations.full_state_sets == enemy_animations.total, "not every enemy transitions through nine animation states")
			_check(enemy_animations.cast_animated == enemy_animations.total, "enemy cast animation is frozen")
			_check(enemy_animations.pain_animated == enemy_animations.total, "enemy pain animation is frozen")
			_check(enemy_animations.one_shots == enemy_animations.total, "enemy attack one-shot does not clamp")
			_check(enemy_animations.timing_profiles >= 3, "enemy roster lacks distinct animation timing profiles")
			print("RUNTIME_SMOKE enemy_animations total=%s states=%s profiles=%s" % [
				enemy_animations.total,
				enemy_animations.full_state_sets,
				enemy_animations.timing_profiles,
			])
		var hazard_trigger: Dictionary = game.runtime_smoke_trigger_hazard()
		_check(hazard_trigger.found, "generated dungeon has no hazard at depth %s" % depth)
		_check(hazard_trigger.damage > 0.0, "hazard caused no damage at depth %s" % depth)
		_check(hazard_trigger.status_applied, "hazard status was not applied at depth %s" % depth)
		if depth == 3:
			var dungeon_guidance: Dictionary = game.runtime_smoke_dungeon_quest_navigation()
			_check(dungeon_guidance.target, "dungeon quest guidance has no target")
			_check(dungeon_guidance.points_to_echo, "lore quest guidance does not point to an unused story echo")
			_check(dungeon_guidance.localized, "dungeon quest guidance is not localized")
			_check(dungeon_guidance.marker_visible, "dungeon quest world marker is hidden")
			var events: Dictionary = game.runtime_smoke_dungeon_events()
			_check(events.puzzle_found, "puzzle event group is missing")
			_check(events.wrong_reset, "wrong puzzle order did not reset relays")
			_check(events.puzzle_solved, "correct puzzle order did not solve relays")
			_check(events.story_found, "story echo event is missing")
			_check(events.story_used, "story echo was not consumed")
			_check(events.gold_gained, "puzzle reward granted no gold")
			_check(events.xp_gained, "dungeon events granted no XP")
		var boss: Dictionary = game.runtime_smoke_probe_boss()
		_check(boss.boss_found, "boss is missing at depth %s" % depth)
		_check(boss.boss_id == expected_bosses[depth - 1], "wrong boss at depth %s: %s" % [depth, boss.boss_id])
		_check(boss.tactical_role == expected_boss_roles[depth - 1], "wrong boss role at depth %s: %s" % [depth, boss.tactical_role])
		_check(boss.telegraph_scale >= 1.0, "boss telegraph scale is invalid at depth %s" % depth)
		_check(boss.phase_count >= 2, "boss has fewer than two phases at depth %s" % depth)
		_check(boss.middle_phase >= 1, "boss first phase did not trigger at depth %s" % depth)
		_check(boss.final_phase >= 2, "boss final phase did not trigger at depth %s" % depth)
		_check(boss.phase_events == 2, "boss did not emit two phase transition events at depth %s" % depth)
		_check(boss.phase_animation, "boss phase animation did not start at depth %s" % depth)
		_check(boss.phase_fx >= 10, "boss phase transitions produced too few visual wave lights at depth %s" % depth)

		if depth == 1:
			var discovery: Dictionary = game.runtime_smoke_pick_weapon()
			_check(discovery.weapon_found, "weapon cache pickup is missing")
			_check(discovery.discover_progress == 1, "weapon discovery quest event failed")

		var combat: Dictionary = game.runtime_smoke_kill_enemy(depth == 4)
		_check(combat.target_found, "no combat target at depth %s" % depth)
		_check(combat.damage > 0.0, "damage pipeline failed at depth %s" % depth)
		_check(combat.enemy_removed, "death pipeline failed at depth %s" % depth)
		_check(combat.xp_gained, "XP reward failed at depth %s" % depth)
		if depth == 4:
			_check(combat.boss_quest_progress == 1, "boss quest event failed")
			_check(combat.boss_core_gained, "boss did not award a weapon mod core")
			_check(combat.boss_flag_set, "boss victory flag was not persisted")
			_check(combat.boss_trophy_gained, "boss did not award its unique trophy")
			_check(combat.boss_memorial_spawned, "boss victory did not alter the hub")
		_check(game.runtime_smoke_exit_dungeon(), "dungeon cleanup failed at depth %s" % depth)

		print("RUNTIME_SMOKE depth=%s boss=%s phases=%s rooms=%s floor=%s enemies=%s items=%s hazards=%s" % [
			depth,
			boss.boss_id,
			boss.final_phase,
			dungeon.room_count,
			dungeon.floor_cells,
			dungeon.enemies,
			dungeon.items,
			dungeon.hazards,
		])

	var skills: Dictionary = game.runtime_smoke_player_skills()
	_check(skills.activated, "player skill did not activate")
	_check(skills.persisted, "ability recovery was lost on save")
	_check(skills.icons == 2, "ability HUD is incomplete")
	_check(skills.perks == 21, "interactive perk grid is incomplete")
	print("RUNTIME_SMOKE player_skills activated=%s persisted=%s perk_cards=%s" % [skills.activated, skills.persisted, skills.perks])
	game.runtime_smoke_shutdown_audio()
	game.queue_free()
	await process_frame
	var music := root.get_node_or_null("/root/Music")
	_check(music != null, "Music autoload is missing")
	if music != null:
		music.shutdown()
	await create_timer(0.25).timeout
	for _frame in 3:
		await process_frame
	quit(1 if failed else 0)
