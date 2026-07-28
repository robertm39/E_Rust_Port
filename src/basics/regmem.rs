//! Safe registered-memory compatibility helpers.
//!
//! C exposes untyped registered pointers whose allocation contents are
//! initially uninitialized. Rust uses opaque handles and initialized byte
//! buffers so safe callers can never observe uninitialized memory. The one C
//! production owner that needs typed persistent scratch storage wraps the same
//! doubling policy at its `Vec<i64>` ownership boundary in `freqvectors`.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::basics::size_class_allocator::try_reserve_exact_vec;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RegMemHandle(usize);

impl RegMemHandle {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegMemError {
    UnknownHandle(RegMemHandle),
    AllocationFailed { size: usize },
    SizeOverflow { old_size: usize, new_size: usize },
}

#[derive(Debug, Default)]
struct RegMemRegistry {
    next_handle: usize,
    buffers: BTreeMap<RegMemHandle, Vec<u8>>,
}

impl RegMemRegistry {
    fn insert(&mut self, buffer: Vec<u8>) -> Result<RegMemHandle, RegMemError> {
        let Some(next) = self.next_handle.checked_add(1) else {
            return Err(RegMemError::SizeOverflow {
                old_size: self.next_handle,
                new_size: usize::MAX,
            });
        };
        let handle = RegMemHandle(self.next_handle);
        self.next_handle = next;
        self.buffers.insert(handle, buffer);
        Ok(handle)
    }
}

static REGISTRY: OnceLock<Mutex<RegMemRegistry>> = OnceLock::new();

fn registry() -> &'static Mutex<RegMemRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(RegMemRegistry::default()))
}

fn lock_registry() -> MutexGuard<'static, RegMemRegistry> {
    match registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn zeroed_buffer(size: usize) -> Result<Vec<u8>, RegMemError> {
    let mut buffer = Vec::new();
    if !try_reserve_exact_vec(&mut buffer, size) {
        return Err(RegMemError::AllocationFailed { size });
    }
    buffer.resize(size, 0);
    Ok(buffer)
}

pub(crate) fn regmem_doubled_limit(old_size: usize, new_size: usize) -> Result<usize, RegMemError> {
    let mut new_limit = old_size.max(1);
    while new_limit < new_size {
        let Some(next) = new_limit.checked_mul(2) else {
            return Err(RegMemError::SizeOverflow { old_size, new_size });
        };
        new_limit = next;
    }
    Ok(new_limit)
}

fn regmem_failure(operation: &str, error: RegMemError) -> ! {
    match error {
        RegMemError::UnknownHandle(handle) => {
            panic!("{operation} called for unregistered handle {handle:?}");
        }
        RegMemError::AllocationFailed { size } => {
            panic!("{operation} failed to allocate {size} bytes");
        }
        RegMemError::SizeOverflow { old_size, new_size } => {
            panic!("{operation} size overflow growing from {old_size} to {new_size}");
        }
    }
}

fn require_regmem<T>(operation: &str, result: Result<T, RegMemError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => regmem_failure(operation, error),
    }
}

#[must_use]
pub fn regmem_registered_count() -> usize {
    lock_registry().buffers.len()
}

pub fn try_regmem_alloc(size: usize) -> Result<RegMemHandle, RegMemError> {
    let buffer = zeroed_buffer(size)?;
    lock_registry().insert(buffer)
}

#[must_use]
pub fn regmem_alloc(size: usize) -> RegMemHandle {
    require_regmem("RegMemAlloc", try_regmem_alloc(size))
}

pub fn try_regmem_realloc(
    handle: Option<RegMemHandle>,
    size: usize,
) -> Result<RegMemHandle, RegMemError> {
    match handle {
        Some(handle) => try_regmem_realloc_preserving(handle, size, usize::MAX),
        None => try_regmem_alloc(size),
    }
}

#[must_use]
pub fn regmem_realloc(handle: Option<RegMemHandle>, size: usize) -> RegMemHandle {
    require_regmem("RegMemRealloc", try_regmem_realloc(handle, size))
}

fn try_regmem_realloc_preserving(
    handle: RegMemHandle,
    size: usize,
    prefix_limit: usize,
) -> Result<RegMemHandle, RegMemError> {
    let mut registry = lock_registry();
    let Some(old_buffer) = registry.buffers.remove(&handle) else {
        return Err(RegMemError::UnknownHandle(handle));
    };

    let mut new_buffer = match zeroed_buffer(size) {
        Ok(buffer) => buffer,
        Err(error) => {
            registry.buffers.insert(handle, old_buffer);
            return Err(error);
        }
    };
    let copy_len = old_buffer.len().min(size).min(prefix_limit);
    new_buffer[..copy_len].copy_from_slice(&old_buffer[..copy_len]);
    let new_handle = registry.insert(new_buffer);
    if new_handle.is_err() {
        registry.buffers.insert(handle, old_buffer);
    }
    new_handle
}

pub fn try_regmem_free(handle: RegMemHandle) -> Result<(), RegMemError> {
    if lock_registry().buffers.remove(&handle).is_some() {
        Ok(())
    } else {
        Err(RegMemError::UnknownHandle(handle))
    }
}

pub fn regmem_free(handle: RegMemHandle) {
    require_regmem("RegMemFree", try_regmem_free(handle));
}

pub fn try_regmem_provide(
    handle: Option<RegMemHandle>,
    old_size: &mut usize,
    new_size: usize,
) -> Result<Option<RegMemHandle>, RegMemError> {
    if *old_size >= new_size {
        return Ok(handle);
    }

    let new_limit = regmem_doubled_limit(*old_size, new_size)?;
    let new_handle = match handle {
        Some(handle) => try_regmem_realloc_preserving(handle, new_limit, *old_size)?,
        None => try_regmem_alloc(new_limit)?,
    };
    *old_size = new_limit;
    Ok(Some(new_handle))
}

#[must_use]
pub fn regmem_provide(
    handle: Option<RegMemHandle>,
    old_size: &mut usize,
    new_size: usize,
) -> Option<RegMemHandle> {
    require_regmem(
        "RegMemProvide",
        try_regmem_provide(handle, old_size, new_size),
    )
}

#[must_use]
pub fn regmem_cleanup() -> usize {
    let mut registry = lock_registry();
    let freed = registry.buffers.len();
    registry.buffers.clear();
    freed
}

pub fn regmem_buffer_len(handle: RegMemHandle) -> Result<usize, RegMemError> {
    lock_registry()
        .buffers
        .get(&handle)
        .map(Vec::len)
        .ok_or(RegMemError::UnknownHandle(handle))
}

pub fn regmem_with_bytes<R>(
    handle: RegMemHandle,
    visit: impl FnOnce(&[u8]) -> R,
) -> Result<R, RegMemError> {
    let registry = lock_registry();
    registry
        .buffers
        .get(&handle)
        .map(|buffer| visit(buffer))
        .ok_or(RegMemError::UnknownHandle(handle))
}

pub fn regmem_with_bytes_mut<R>(
    handle: RegMemHandle,
    visit: impl FnOnce(&mut [u8]) -> R,
) -> Result<R, RegMemError> {
    let mut registry = lock_registry();
    registry
        .buffers
        .get_mut(&handle)
        .map(|buffer| visit(buffer))
        .ok_or(RegMemError::UnknownHandle(handle))
}

#[cfg(test)]
mod tests {
    use super::{
        regmem_alloc, regmem_buffer_len, regmem_cleanup, regmem_free, regmem_provide,
        regmem_realloc, regmem_registered_count, regmem_with_bytes, regmem_with_bytes_mut,
        try_regmem_free, try_regmem_provide, try_regmem_realloc, RegMemError, RegMemHandle,
    };
    use std::sync::{Mutex, OnceLock};

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        match LOCK.get_or_init(|| Mutex::new(())).lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[test]
    fn alloc_registers_zeroed_memory_and_free_unregisters_it() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let handle = regmem_alloc(4);
        assert_eq!(regmem_registered_count(), 1);
        assert_eq!(regmem_buffer_len(handle), Ok(4));
        assert_eq!(
            regmem_with_bytes(handle, <[u8]>::to_vec).unwrap(),
            vec![0; 4]
        );

        regmem_free(handle);
        assert_eq!(regmem_registered_count(), 0);
        assert_eq!(
            try_regmem_free(handle),
            Err(RegMemError::UnknownHandle(handle))
        );
    }

    #[test]
    #[should_panic(expected = "RegMemFree called for unregistered handle")]
    fn free_panics_for_unknown_handle() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        regmem_free(RegMemHandle(17));
    }

    #[test]
    fn realloc_preserves_prefix_and_invalidates_old_handle() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let handle = regmem_alloc(3);
        regmem_with_bytes_mut(handle, |buffer| buffer.copy_from_slice(&[1, 2, 3])).unwrap();

        let grown = regmem_realloc(Some(handle), 5);
        assert_ne!(grown, handle);
        assert_eq!(
            regmem_with_bytes(grown, <[u8]>::to_vec).unwrap(),
            vec![1, 2, 3, 0, 0]
        );
        assert_eq!(
            regmem_buffer_len(handle),
            Err(RegMemError::UnknownHandle(handle))
        );

        let shrunk = regmem_realloc(Some(grown), 2);
        assert_eq!(
            regmem_with_bytes(shrunk, <[u8]>::to_vec).unwrap(),
            vec![1, 2]
        );
        assert_eq!(regmem_cleanup(), 1);
    }

    #[test]
    #[should_panic(expected = "RegMemRealloc called for unregistered handle")]
    fn realloc_panics_for_unknown_handle() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let _ = regmem_realloc(Some(RegMemHandle(18)), 3);
    }

    #[test]
    fn try_realloc_reports_unknown_handle() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let handle = RegMemHandle(19);
        assert_eq!(
            try_regmem_realloc(Some(handle), 3),
            Err(RegMemError::UnknownHandle(handle))
        );
        assert_eq!(regmem_registered_count(), 0);
    }

    #[test]
    fn provide_doubles_capacity_and_zeroes_new_tail() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let mut old_size = 0;
        let handle = regmem_provide(None, &mut old_size, 5).unwrap();
        assert_eq!(old_size, 8);
        assert_eq!(regmem_buffer_len(handle), Ok(8));
        regmem_with_bytes_mut(handle, |buffer| buffer[..3].copy_from_slice(&[9, 8, 7])).unwrap();

        let same = regmem_provide(Some(handle), &mut old_size, 7);
        assert_eq!(same, Some(handle));
        assert_eq!(old_size, 8);

        let grown = regmem_provide(Some(handle), &mut old_size, 9).unwrap();
        assert_eq!(old_size, 16);
        assert_ne!(grown, handle);
        assert_eq!(
            regmem_with_bytes(grown, <[u8]>::to_vec).unwrap(),
            vec![9, 8, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(regmem_cleanup(), 1);
    }

    #[test]
    fn provide_overflow_is_recoverable_and_leaves_ownership_unchanged() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();
        let mut old_size = usize::MAX / 2 + 1;

        assert_eq!(
            try_regmem_provide(None, &mut old_size, usize::MAX),
            Err(RegMemError::SizeOverflow {
                old_size,
                new_size: usize::MAX,
            })
        );
        assert_eq!(old_size, usize::MAX / 2 + 1);
        assert_eq!(regmem_registered_count(), 0);
    }
}
