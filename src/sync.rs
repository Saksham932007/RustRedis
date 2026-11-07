use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locking that survives poisoning.
///
/// A mutex is poisoned when a thread panics while holding it. With a plain
/// `lock().unwrap()` every later caller would then panic too, turning one bug
/// into permanent unavailability. The guarded data in this crate is always left
/// in a consistent state between operations, so it is safe to keep serving it.
pub(crate) trait LockExt<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
