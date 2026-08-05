# World, quest and dungeon content plan

## Quest structure

Quest content is split into chains rather than a flat task list:

- District recovery: `neon_shards` → `stolen_cores` → `drone_blackout`.
- Hunter trail: `cull_grunts` → `shade_hunt` → `hound_pack`.
- Medical relief: `medkit_run` → `vermin_sweep`.
- Heart descent: `depth_two` → `void_liturgy` → `heart_descent`.

Every quest may declare `requires` plus separate `offer_scene`, `progress_scene`, and
`complete_scene`. New chains should mix combat, collection, exploration, survival,
escort, defense, interaction, puzzle and choice objectives instead of repeating kills.

## Dialogue direction

- Give every important NPC a voice, goal, fear and relationship to the district.
- Use dedicated scenes at chain milestones; generated dialogue remains a fallback.
- Progress dialogue should teach mechanics or point to landmarks, not repeat counters.
- Choices should affect flags, relationships, rewards, routes or later quest availability.
- Major chain endings should change NPC placement, lighting, enemies or district props.

## Neon Quarter composition

The hub should be built in six visual layers:

1. **Primary silhouette** — gate, heart tower, elevated transit line and district walls.
2. **Readable routes** — central boulevard, two looping side streets and vertical shortcuts.
3. **Landmarks** — clinic, drone workshop, hunter den, archive, market and descent shrine.
4. **Mid-scale clutter** — kiosks, bridges, awnings, pipes, cables, barricades and balconies.
5. **Micro detail** — signs, posters, vents, crates, trash, puddles, decals and vegetation.
6. **Atmosphere** — local fog volumes, contrasting light pools, particles and animated emissives.

Each district cluster needs its own silhouette, material trio and light color. Empty ground
larger than eight metres should be broken by elevation, a prop cluster, a route split or a
deliberate combat arena. Repeated objects should use rotation, scale and material variants.

## Dungeon room library

`dungeon.json` now controls sixteen weighted room archetypes across ten gameplay roles.
Alongside service cells, ritual rotundas, vaults, shrines, galleries and arenas, the library
includes a vertical cistern, relay-lock puzzle, memory tableau and boss antechamber.

Gameplay roles remain independent from geometry:

- entry / safe room;
- arena / ambush;
- ranged gallery;
- ritual / summoning;
- flooded hazard;
- traversal / vertical room;
- treasure vault;
- puzzle / switch room;
- story tableau;
- boss antechamber.

Each role has a distinct set piece, lighting language, combat pressure and enemy-role filter.
Generation enforces a safe entry, reward and ritual beats, a boss antechamber on every depth,
traversal and puzzle beats from depth two, and a non-combat story beat from depth three.
Consecutive random rooms also avoid repeating the same role.

Hazard zones now have distinct counter-play. Blood pools are continuously dangerous, void
rifts and electric plates alternate warning/dormant/active phases, and ember floors flare in
cycles. Each profile applies a matching status and emits a light pulse before activation.

The dungeon minimap now renders outlined walkable routes, role-colored room centers, hazard
types and both portals. A rotating player marker communicates heading, changes color inside
active danger, and a compact localized legend explains the visual hierarchy.

Active quests now share one runtime navigation resolver. `T` cycles tracked objectives; the HUD
shows localized title, full objective text, progress and live distance. A pulsing 3D marker follows
the real target NPC, enemy, pickup, portal, relay, memory echo or boss. Inside dungeons the same
target is baked into the semantic minimap, so world and map guidance cannot disagree.

`J` opens a dedicated bilingual quest journal. It groups all twenty stages into five chains, shows
chain completion, active/completed/locked states, live progress, rewards and full objective text.
Keys `1–5` select an active entry for the same HUD/world/minimap tracking pipeline.

Puzzle and story rooms are now interactive rather than decorative. Puzzle rooms spawn three
color-coded relays that must be synchronized in order; a wrong input resets the group, while
completion grants gold and XP. Story rooms contain a one-use memory echo with depth-specific
Russian and English lore, XP and a quest event hook.

Those hooks now drive a two-part Scientist quest chain. `Relay Resonance` asks for two solved
relay groups after the drone blackout, then unlocks `Whispers of the Archive`, which requires
two memory echoes. Both quests have authored offer/progress/completion dialogue in Russian and
English, exact validated rewards and dependency-aware runtime completion coverage.

Enemy animation now uses one state-driven animator for idle, directional movement, alert,
attack, pain, cast, charge and death. Early AI returns no longer freeze cast/stagger/charge
frames; attacks and deaths are one-shots, ability telegraphs loop, and each state adds subtle
bob, recoil, pulse or collapse motion. Per-state FPS and hit timing are preset-configurable,
with distinct defaults for all seven tactical roles and authored boss profiles.

Boss phase changes are now encounter events rather than silent stat mutations. Every threshold
starts a dedicated ninth `Phase` animation, pauses attacks for a readable 0.9-second window,
announces the phase in Russian or English, changes tint and emits a central burst plus four
light-wave fronts. The transition queue is processed by Game3D alongside ability requests and
is covered for both thresholds of all four dungeon bosses.
