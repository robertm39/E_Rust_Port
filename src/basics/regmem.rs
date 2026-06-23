use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

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
    if buffer.try_reserve_exact(size).is_err() {
        return Err(RegMemError::AllocationFailed { size });
    }
    buffer.resize(size, 0);
    Ok(buffer)
}

fn doubled_limit(old_size: usize, new_size: usize) -> Result<usize, RegMemError> {
    let mut new_limit = old_size.max(1);
    while new_limit < new_size {
        let Some(next) = new_limit.checked_mul(2) else {
            return Err(RegMemError::SizeOverflow { old_size, new_size });
        };
        new_limit = next;
    }
    Ok(new_limit)
}

#[must_use]
pub fn regmem_registered_count() -> usize {
    lock_registry().buffers.len()
}

pub fn regmem_alloc(size: usize) -> Result<RegMemHandle, RegMemError> {
    let buffer = zeroed_buffer(size)?;
    lock_registry().insert(buffer)
}

pub fn regmem_realloc(
    handle: Option<RegMemHandle>,
    size: usize,
) -> Result<RegMemHandle, RegMemError> {
    match handle {
        Some(handle) => regmem_realloc_preserving(handle, size, usize::MAX),
        None => regmem_alloc(size),
    }
}

fn regmem_realloc_preserving(
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
    registry.insert(new_buffer)
}

pub fn regmem_free(handle: RegMemHandle) -> Result<(), RegMemError> {
    if lock_registry().buffers.remove(&handle).is_some() {
        Ok(())
    } else {
        Err(RegMemError::UnknownHandle(handle))
    }
}

pub fn regmem_provide(
    handle: Option<RegMemHandle>,
    old_size: &mut usize,
    new_size: usize,
) -> Result<Option<RegMemHandle>, RegMemError> {
    if *old_size >= new_size {
        return Ok(handle);
    }

    let new_limit = doubled_limit(*old_size, new_size)?;
    let new_handle = match handle {
        Some(handle) => regmem_realloc_preserving(handle, new_limit, *old_size)?,
        None => regmem_alloc(new_limit)?,
    };
    *old_size = new_limit;
    Ok(Some(new_handle))
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
        RegMemError,
    };
    use std::sync::{Mutex, OnceLock};

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn alloc_registers_zeroed_memory_and_free_unregisters_it() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let handle = regmem_alloc(4).unwrap();
        assert_eq!(regmem_registered_count(), 1);
        assert_eq!(regmem_buffer_len(handle), Ok(4));
        assert_eq!(
            regmem_with_bytes(handle, <[u8]>::to_vec).unwrap(),
            vec![0; 4]
        );

        assert_eq!(regmem_free(handle), Ok(()));
        assert_eq!(regmem_registered_count(), 0);
        assert_eq!(regmem_free(handle), Err(RegMemError::UnknownHandle(handle)));
    }

    #[test]
    fn realloc_preserves_prefix_and_invalidates_old_handle() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let handle = regmem_alloc(3).unwrap();
        regmem_with_bytes_mut(handle, |buffer| buffer.copy_from_slice(&[1, 2, 3])).unwrap();

        let grown = regmem_realloc(Some(handle), 5).unwrap();
        assert_ne!(grown, handle);
        assert_eq!(
            regmem_with_bytes(grown, <[u8]>::to_vec).unwrap(),
            vec![1, 2, 3, 0, 0]
        );
        assert_eq!(
            regmem_buffer_len(handle),
            Err(RegMemError::UnknownHandle(handle))
        );

        let shrunk = regmem_realloc(Some(grown), 2).unwrap();
        assert_eq!(
            regmem_with_bytes(shrunk, <[u8]>::to_vec).unwrap(),
            vec![1, 2]
        );
        assert_eq!(regmem_cleanup(), 1);
    }

    #[test]
    fn provide_doubles_capacity_and_zeroes_new_tail() {
        let _guard = global_test_lock();
        let _ = regmem_cleanup();

        let mut old_size = 0;
        let handle = regmem_provide(None, &mut old_size, 5).unwrap().unwrap();
        assert_eq!(old_size, 8);
        assert_eq!(regmem_buffer_len(handle), Ok(8));
        regmem_with_bytes_mut(handle, |buffer| buffer[..3].copy_from_slice(&[9, 8, 7])).unwrap();

        let same = regmem_provide(Some(handle), &mut old_size, 7).unwrap();
        assert_eq!(same, Some(handle));
        assert_eq!(old_size, 8);

        let grown = regmem_provide(Some(handle), &mut old_size, 9)
            .unwrap()
            .unwrap();
        assert_eq!(old_size, 16);
        assert_ne!(grown, handle);
        assert_eq!(
            regmem_with_bytes(grown, <[u8]>::to_vec).unwrap(),
            vec![9, 8, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(regmem_cleanup(), 1);
    }
}
