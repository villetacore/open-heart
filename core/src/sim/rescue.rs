//! Даун и ревайв: в кооперативе смерть одного — не конец забега.
//!
//! Игрок с нулевым здоровьем не умирает сразу, а ложится: он не цель для врагов,
//! не может стрелять, но товарищ может поднять его, простояв рядом несколько
//! секунд. Забег заканчивается, только когда лежат все (docs/MULTIPLAYER.md §11).

use crate::sim::{buttons, event, flags, Event, Player, State};

/// Сколько игрок истекает кровью, прежде чем выбывает окончательно.
pub const BLEEDOUT: f32 = 45.0;
/// Сколько нужно простоять рядом, чтобы поднять.
pub const REVIVE_TIME: f32 = 5.0;
/// На каком расстоянии можно поднимать.
pub const REVIVE_RANGE: f32 = 2.5;
/// С какой долей здоровья поднимается игрок.
pub const REVIVE_HP_FRAC: f32 = 0.3;
/// Как быстро откатывается прогресс, если спасатель отошёл.
const PROGRESS_DECAY: f32 = 2.0;

/// Шаг спасения: подъём, кровотечение, проверка вайпа.
pub fn tick(state: &mut State, dt: f32) -> Vec<Event> {
    let mut events = Vec::new();

    // Кто может поднимать: живой, не лежит, держит «использовать».
    let rescuers: Vec<(u16, crate::math::Vec3, bool)> = state
        .players
        .values()
        .filter(|player| !player.downed() && !player.out)
        .map(|player| (player.peer, player.pos, player.buttons & buttons::USE != 0))
        .collect();

    let downed: Vec<u16> = state
        .players
        .values()
        .filter(|player| player.downed() && !player.out)
        .map(|player| player.peer)
        .collect();

    for peer in downed {
        let Some(player) = state.players.get(&peer) else {
            continue;
        };
        let position = player.pos;

        // Спасатель: ближайший, кто рядом и держит кнопку.
        let helper = rescuers
            .iter()
            .filter(|(_, pos, holding)| *holding && (*pos - position).length() <= REVIVE_RANGE)
            .min_by(|a, b| {
                let da = (a.1 - position).length();
                let db = (b.1 - position).length();
                da.total_cmp(&db)
            })
            .map(|(peer, _, _)| *peer);

        let Some(player) = state.players.get_mut(&peer) else {
            continue;
        };

        match helper {
            Some(helper_peer) => {
                player.revive_progress += dt;
                if player.revive_progress >= REVIVE_TIME {
                    player.hp = player.max_hp * REVIVE_HP_FRAC;
                    player.flags &= !flags::DOWNED;
                    player.downed_time = 0.0;
                    player.revive_progress = 0.0;

                    let mut revived = Event::new(event::REVIVED);
                    revived.actor = helper_peer;
                    revived.target = peer;
                    revived.pos = Some(player.pos);
                    events.push(revived);
                }
            }
            None => {
                // Без спасателя прогресс тает, а время истекает.
                player.revive_progress = (player.revive_progress - dt * PROGRESS_DECAY).max(0.0);
                player.downed_time += dt;
                if player.downed_time >= BLEEDOUT {
                    player.out = true;
                    let mut lost = Event::new(event::PLAYER_OUT);
                    lost.target = peer;
                    lost.pos = Some(player.pos);
                    events.push(lost);
                }
            }
        }
    }

    if let Some(wipe) = check_wipe(state) {
        events.push(wipe);
    }
    events
}

/// Забег провален, когда никто не стоит на ногах.
fn check_wipe(state: &mut State) -> Option<Event> {
    if state.players.is_empty() || state.wiped {
        return None;
    }
    let anyone_up = state.players.values().any(|player| !player.downed() && !player.out);
    if anyone_up {
        return None;
    }
    state.wiped = true;
    Some(Event::new(event::WIPE))
}

/// Пометить игрока лежащим — вызывается, когда здоровье дошло до нуля.
pub fn knock_down(player: &mut Player) {
    player.flags |= flags::DOWNED;
    player.downed_time = 0.0;
    player.revive_progress = 0.0;
}
