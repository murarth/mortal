use std::sync::{LockResult, PoisonError, TryLockError, TryLockResult};

use crate::screen::{Screen, ScreenReadGuard};
use crate::terminal::{Terminal, TerminalReadGuard};
use crate::util::char_width;

// Private trait used to prevent external crates from implementing extension traits
pub trait Private {}

impl Private for Screen {}
impl<'a> Private for ScreenReadGuard<'a> {}
impl Private for Terminal {}
impl<'a> Private for TerminalReadGuard<'a> {}

pub fn is_visible(ch: char) -> bool {
    match ch {
        '\t' | '\r' | '\n' => true,
        _ => char_width(ch).unwrap_or(0) != 0
    }
}

pub fn map_lock_result<F, T, U>(res: LockResult<T>, f: F) -> LockResult<U>
        where F: FnOnce(T) -> U {
    match res {
        Ok(t) => Ok(f(t)),
        Err(e) => Err(PoisonError::new(f(e.into_inner()))),
    }
}

pub fn map_try_lock_result<F, T, U>(res: TryLockResult<T>, f: F) -> TryLockResult<U>
        where F: FnOnce(T) -> U {
    match res {
        Ok(t) => Ok(f(t)),
        Err(TryLockError::Poisoned(p)) => Err(TryLockError::Poisoned(
            PoisonError::new(f(p.into_inner())))),
        Err(TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
    }
}

pub fn map2_lock_result<F, T, U, R>(res: LockResult<T>, res2: LockResult<U>, f: F)
        -> LockResult<R> where F: FnOnce(T, U) -> R {
    match (res, res2) {
        (Ok(a), Ok(b)) => Ok(f(a, b)),
        (Ok(a), Err(b)) => Err(PoisonError::new(f(a, b.into_inner()))),
        (Err(a), Ok(b)) => Err(PoisonError::new(f(a.into_inner(), b))),
        (Err(a), Err(b)) => Err(PoisonError::new(f(a.into_inner(), b.into_inner()))),
    }
}

pub fn map2_try_lock_result<F, T, U, R>(
        res: TryLockResult<T>, res2: TryLockResult<U>, f: F)
        -> TryLockResult<R> where F: FnOnce(T, U) -> R {
    match (res, res2) {
        (Ok(a), Ok(b)) => Ok(f(a, b)),
        (Err(TryLockError::WouldBlock), _) => Err(TryLockError::WouldBlock),
        (_, Err(TryLockError::WouldBlock)) => Err(TryLockError::WouldBlock),
        (Ok(a), Err(TryLockError::Poisoned(b))) =>
            Err(TryLockError::Poisoned(PoisonError::new(f(a, b.into_inner())))),
        (Err(TryLockError::Poisoned(a)), Ok(b)) =>
            Err(TryLockError::Poisoned(PoisonError::new(f(a.into_inner(), b)))),
        (Err(TryLockError::Poisoned(a)), Err(TryLockError::Poisoned(b))) =>
            Err(TryLockError::Poisoned(PoisonError::new(
                f(a.into_inner(), b.into_inner())))),
    }
}
