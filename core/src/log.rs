//! Предупреждения из ядра.
//!
//! Ядро не знает, кто его исполняет: в клиенте это движок (`godot_warn!`),
//! на сервере — `slog`, в тестах — stderr. Хост ставит хук один раз при старте,
//! ядро зовёт [`warn`] и не думает о том, куда это попадёт.

use std::sync::RwLock;

type Hook = fn(&str);

static HOOK: RwLock<Option<Hook>> = RwLock::new(None);

/// Поставить приёмник предупреждений. Клиент вызывает это при инициализации
/// расширения, сервер — при старте процесса.
pub fn set_hook(hook: Hook) {
    if let Ok(mut slot) = HOOK.write() {
        *slot = Some(hook);
    }
}

/// Сообщить о проблеме, которая не мешает продолжать (битый JSON, неизвестный id).
pub fn warn(message: &str) {
    let hook = HOOK.read().ok().and_then(|slot| *slot);
    match hook {
        Some(hook) => hook(message),
        // Без хука (тесты, инструменты) — в stderr, чтобы предупреждение не пропало.
        None => eprintln!("[openheart-core] {message}"),
    }
}

/// Предупреждение с форматированием: замена `godot_warn!` внутри ядра.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log::warn(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);

    fn count(_msg: &str) {
        CALLS.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn hook_receives_warnings() {
        set_hook(count);
        let before = CALLS.load(Ordering::SeqCst);
        crate::warn!("тест {}", 1);
        assert_eq!(CALLS.load(Ordering::SeqCst), before + 1);
    }
}
