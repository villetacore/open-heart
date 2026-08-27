//! C-ABI ядра для wasm-хоста (`oh-server` через wazero).
//!
//! Соглашения (docs/MULTIPLAYER.md §7.2):
//! * буферы передаются через линейную память: хост зовёт [`sim_alloc`], пишет туда
//!   JSON и передаёт `(ptr, len)`;
//! * результат возвращается упакованным: `(ptr as u64) << 32 | len as u64`,
//!   хост читает его и обязан освободить через [`sim_free`];
//! * `0` в качестве результата означает «пусто» (для `sim_new` — ошибка).
//!
//! Вызовы крупноблочные — один на тик, а не на сущность: пересечение границы wasm
//! стоит дороже самой симуляции.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::sim::{Cmd, Config, State, TaggedInput};

thread_local! {
    static ROOMS: RefCell<HashMap<u32, State>> = RefCell::new(HashMap::new());
    static NEXT_HANDLE: RefCell<u32> = const { RefCell::new(1) };
}

/// Выделить буфер под данные хоста.
///
/// # Safety
/// Возвращённый указатель освобождает [`sim_free`] с той же длиной.
#[no_mangle]
pub extern "C" fn sim_alloc(len: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    // Именно vec![0; len], а не with_capacity: sim_free восстанавливает Vec
    // с capacity == len, поэтому длина и ёмкость обязаны совпадать.
    let mut buf = vec![0u8; len as usize];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr as u32
}

/// Освободить буфер, ранее выданный [`sim_alloc`] или возвращённый ядром.
///
/// # Safety
/// `ptr` и `len` должны быть той же парой, что вернуло ядро.
#[no_mangle]
pub unsafe extern "C" fn sim_free(ptr: u32, len: u32) {
    if ptr == 0 || len == 0 {
        return;
    }
    drop(Vec::from_raw_parts(ptr as *mut u8, len as usize, len as usize));
}

/// Создать комнату из JSON-конфига. 0 = ошибка.
///
/// # Safety
/// `(cfg_ptr, cfg_len)` — валидный буфер в памяти модуля.
#[no_mangle]
pub unsafe extern "C" fn sim_new(cfg_ptr: u32, cfg_len: u32) -> u32 {
    let Some(cfg) = read_json::<Config>(cfg_ptr, cfg_len) else {
        return 0;
    };
    let handle = NEXT_HANDLE.with(|h| {
        let mut h = h.borrow_mut();
        let cur = *h;
        *h = h.wrapping_add(1).max(1);
        cur
    });
    ROOMS.with(|rooms| rooms.borrow_mut().insert(handle, State::new(cfg)));
    handle
}

/// Уничтожить комнату.
#[no_mangle]
pub extern "C" fn sim_drop(handle: u32) {
    ROOMS.with(|rooms| rooms.borrow_mut().remove(&handle));
}

/// Шаг симуляции. `dt_ms` — миллисекунды, `(inputs_ptr, inputs_len)` — JSON-массив
/// [`TaggedInput`]. Возвращает упакованный JSON [`crate::sim::TickResult`].
///
/// # Safety
/// `(inputs_ptr, inputs_len)` — валидный буфер в памяти модуля.
#[no_mangle]
pub unsafe extern "C" fn sim_tick(handle: u32, dt_ms: u32, inputs_ptr: u32, inputs_len: u32) -> u64 {
    let inputs: Vec<TaggedInput> = read_json(inputs_ptr, inputs_len).unwrap_or_default();
    let dt = dt_ms as f32 / 1000.0;

    ROOMS.with(|rooms| {
        let mut rooms = rooms.borrow_mut();
        let Some(state) = rooms.get_mut(&handle) else {
            return 0;
        };
        let result = state.tick(dt, &inputs);
        write_json(&result)
    })
}

/// Команда ядру (join/leave/fire/hit/cmd). Возвращает упакованный JSON-массив событий.
///
/// # Safety
/// `(msg_ptr, msg_len)` — валидный буфер в памяти модуля.
#[no_mangle]
pub unsafe extern "C" fn sim_cmd(handle: u32, msg_ptr: u32, msg_len: u32) -> u64 {
    let Some(cmd) = read_json::<Cmd>(msg_ptr, msg_len) else {
        return 0;
    };
    ROOMS.with(|rooms| {
        let mut rooms = rooms.borrow_mut();
        let Some(state) = rooms.get_mut(&handle) else {
            return 0;
        };
        let events = state.command(&cmd);
        if events.is_empty() {
            return 0;
        }
        write_json(&events)
    })
}

/// Сериализовать состояние комнаты для персиста.
#[no_mangle]
pub extern "C" fn sim_save(handle: u32) -> u64 {
    ROOMS.with(|rooms| {
        let rooms = rooms.borrow();
        match rooms.get(&handle) {
            Some(state) => write_json(state),
            None => 0,
        }
    })
}

/// Восстановить комнату из блоба [`sim_save`]. 0 = ошибка.
///
/// # Safety
/// `(blob_ptr, blob_len)` — валидный буфер в памяти модуля.
#[no_mangle]
pub unsafe extern "C" fn sim_load(blob_ptr: u32, blob_len: u32) -> u32 {
    let Some(state) = read_json::<State>(blob_ptr, blob_len) else {
        return 0;
    };
    let handle = NEXT_HANDLE.with(|h| {
        let mut h = h.borrow_mut();
        let cur = *h;
        *h = h.wrapping_add(1).max(1);
        cur
    });
    ROOMS.with(|rooms| rooms.borrow_mut().insert(handle, state));
    handle
}

/// Версия протокола, чтобы хост мог проверить совместимость ядра.
#[no_mangle]
pub extern "C" fn sim_protocol_version() -> u32 {
    crate::PROTOCOL_VERSION as u32
}

// ── вспомогательное ──────────────────────────────────────────────────────────

unsafe fn read_json<T: serde::de::DeserializeOwned>(ptr: u32, len: u32) -> Option<T> {
    if ptr == 0 || len == 0 {
        return None;
    }
    let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    serde_json::from_slice(slice).ok()
}

/// Сериализует значение и передаёт владение буфером хосту (тот зовёт `sim_free`).
fn write_json<T: serde::Serialize>(value: &T) -> u64 {
    let Ok(mut data) = serde_json::to_vec(value) else {
        return 0;
    };
    if data.is_empty() {
        return 0;
    }
    data.shrink_to_fit();
    let len = data.len() as u32;
    let ptr = data.as_mut_ptr() as u32;
    std::mem::forget(data);
    ((ptr as u64) << 32) | len as u64
}
