use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::mem;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MEM_ARR_SIZE: usize = 8192;
pub const MEM_ALIGN: usize = 16;
pub const MEM_CHUNKLIMIT: usize = 4096 / MEM_ALIGN;
pub const MEM_MULTIPLIER: usize = 1024;
pub const MEM_FREE_PATTERN: u64 = 0xFAFB_FAFA;
pub const MEM_RSET_PATTERN: u64 = 0x0000_0000;

#[must_use]
pub const fn mem_arr_min_index() -> usize {
    mem::size_of::<usize>()
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MemoryPolicy {
    OldExact,
    NewAligned,
}

impl MemoryPolicy {
    #[must_use]
    pub const fn effective_size(self, requested_size: usize) -> usize {
        match self {
            Self::OldExact => requested_size,
            Self::NewAligned => {
                if requested_size < mem_arr_min_index() {
                    mem_arr_min_index()
                } else {
                    requested_size
                }
            }
        }
    }

    #[must_use]
    pub const fn bucket_size(self, requested_size: usize) -> Option<usize> {
        let effective_size = self.effective_size(requested_size);
        match self {
            Self::OldExact => {
                if effective_size >= mem_arr_min_index() && effective_size < MEM_ARR_SIZE {
                    Some(effective_size)
                } else {
                    None
                }
            }
            Self::NewAligned => {
                let mem_index = effective_size.div_ceil(MEM_ALIGN);
                if mem_index < MEM_ARR_SIZE {
                    Some(mem_index * MEM_ALIGN)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryBlock {
    requested_size: usize,
    allocation_size: usize,
    bytes: Vec<u8>,
}

impl MemoryBlock {
    fn zeroed(requested_size: usize, allocation_size: usize) -> Result<Self, MemoryError> {
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(allocation_size).is_err() {
            return Err(MemoryError::AllocationFailed {
                size: allocation_size,
            });
        }
        bytes.resize(allocation_size, 0);
        Ok(Self {
            requested_size,
            allocation_size,
            bytes,
        })
    }

    #[must_use]
    pub const fn requested_size(&self) -> usize {
        self.requested_size
    }

    #[must_use]
    pub const fn allocation_size(&self) -> usize {
        self.allocation_size
    }

    #[must_use]
    pub fn requested_bytes(&self) -> &[u8] {
        &self.bytes[..self.requested_size.min(self.bytes.len())]
    }

    #[must_use]
    pub fn allocation_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn allocation_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryStats {
    pub mem_is_low: bool,
    pub size_malloc_mem: usize,
    pub size_malloc_count: usize,
    pub size_free_mem: usize,
    pub size_free_count: usize,
    pub clb_free_count: usize,
    pub secure_malloc_count: usize,
    pub secure_malloc_mem: usize,
    pub secure_realloc_count: usize,
    pub secure_realloc_m_count: usize,
    pub secure_realloc_f_count: usize,
    pub flush_count: usize,
    pub free_list_blocks: usize,
    pub free_list_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryError {
    AllocationFailed { size: usize },
    SizeOverflow { size: usize, multiplier: usize },
}

fn memory_failure(operation: &str, error: MemoryError) -> ! {
    match error {
        MemoryError::AllocationFailed { size } => {
            panic!("{operation} failed to allocate {size} bytes");
        }
        MemoryError::SizeOverflow { size, multiplier } => {
            panic!("{operation} size overflow multiplying {size} by {multiplier}");
        }
    }
}

fn require_memory<T>(operation: &str, result: Result<T, MemoryError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => memory_failure(operation, error),
    }
}

#[derive(Debug, Default)]
struct MemoryState {
    stats: MemoryStats,
    free_lists: BTreeMap<(MemoryPolicy, usize), Vec<MemoryBlock>>,
}

impl MemoryState {
    fn free_list_totals(&self) -> (usize, usize) {
        self.free_lists
            .values()
            .flat_map(|blocks| blocks.iter())
            .fold((0, 0), |(count, bytes), block| {
                (
                    count.saturating_add(1),
                    bytes.saturating_add(block.allocation_size()),
                )
            })
    }

    fn stats_snapshot(&self) -> MemoryStats {
        let (free_list_blocks, free_list_bytes) = self.free_list_totals();
        let mut stats = self.stats.clone();
        stats.free_list_blocks = free_list_blocks;
        stats.free_list_bytes = free_list_bytes;
        stats
    }

    fn flush_free_lists(&mut self) -> (usize, usize) {
        let totals = self.free_list_totals();
        self.free_lists.clear();
        self.stats.flush_count = self.stats.flush_count.saturating_add(1);
        totals
    }

    fn secure_malloc_block(
        &mut self,
        requested_size: usize,
        allocation_size: usize,
    ) -> Result<MemoryBlock, MemoryError> {
        self.stats.secure_malloc_count = self.stats.secure_malloc_count.saturating_add(1);
        self.stats.secure_malloc_mem = self.stats.secure_malloc_mem.saturating_add(allocation_size);

        match MemoryBlock::zeroed(requested_size, allocation_size) {
            Ok(block) => Ok(block),
            Err(error) => {
                self.stats.mem_is_low = true;
                self.flush_free_lists();
                MemoryBlock::zeroed(requested_size, allocation_size).map_err(|_| error)
            }
        }
    }

    fn add_newmem_chunk(&mut self, mem_index: usize) -> Result<usize, MemoryError> {
        let Some(block_size) = mem_index.checked_mul(MEM_ALIGN) else {
            return Err(MemoryError::SizeOverflow {
                size: mem_index,
                multiplier: MEM_ALIGN,
            });
        };
        let Some(total_size) = block_size.checked_mul(MEM_MULTIPLIER) else {
            return Err(MemoryError::SizeOverflow {
                size: block_size,
                multiplier: MEM_MULTIPLIER,
            });
        };

        self.stats.secure_malloc_count = self.stats.secure_malloc_count.saturating_add(1);
        self.stats.secure_malloc_mem = self.stats.secure_malloc_mem.saturating_add(total_size);
        let list = self
            .free_lists
            .entry((MemoryPolicy::NewAligned, block_size))
            .or_default();
        for _ in 0..MEM_MULTIPLIER {
            list.push(MemoryBlock::zeroed(block_size, block_size)?);
        }
        Ok(MEM_MULTIPLIER)
    }
}

static MEMORY_STATE: OnceLock<Mutex<MemoryState>> = OnceLock::new();

fn memory_state() -> &'static Mutex<MemoryState> {
    MEMORY_STATE.get_or_init(|| Mutex::new(MemoryState::default()))
}

fn lock_state() -> MutexGuard<'static, MemoryState> {
    match memory_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[must_use]
pub fn memory_stats() -> MemoryStats {
    lock_state().stats_snapshot()
}

#[must_use]
pub fn mem_is_low() -> bool {
    lock_state().stats.mem_is_low
}

#[must_use]
pub fn set_mem_is_low(value: bool) -> bool {
    let mut state = lock_state();
    let old = state.stats.mem_is_low;
    state.stats.mem_is_low = value;
    old
}

#[must_use]
pub fn mem_flush_free_list() -> (usize, usize) {
    lock_state().flush_free_lists()
}

pub fn try_secure_malloc(size: usize) -> Result<MemoryBlock, MemoryError> {
    lock_state().secure_malloc_block(size, size)
}

#[must_use]
pub fn secure_malloc(size: usize) -> MemoryBlock {
    require_memory("SecureMalloc", try_secure_malloc(size))
}

pub fn try_secure_realloc(
    block: Option<MemoryBlock>,
    size: usize,
) -> Result<Option<MemoryBlock>, MemoryError> {
    let mut state = lock_state();
    state.stats.secure_realloc_count = state.stats.secure_realloc_count.saturating_add(1);
    if block.is_some() && size == 0 {
        state.stats.secure_realloc_f_count = state.stats.secure_realloc_f_count.saturating_add(1);
        return Ok(None);
    }
    if block.is_none() && size != 0 {
        state.stats.secure_realloc_m_count = state.stats.secure_realloc_m_count.saturating_add(1);
    }
    if size == 0 {
        return Ok(None);
    }

    let old_block = block;
    let mut new_block = match MemoryBlock::zeroed(size, size) {
        Ok(block) => block,
        Err(error) => {
            state.stats.mem_is_low = true;
            state.flush_free_lists();
            MemoryBlock::zeroed(size, size).map_err(|_| error)?
        }
    };
    if let Some(old_block) = old_block {
        let copy_len = old_block.allocation_size().min(size);
        new_block.allocation_bytes_mut()[..copy_len]
            .copy_from_slice(&old_block.allocation_bytes()[..copy_len]);
    }
    Ok(Some(new_block))
}

#[must_use]
pub fn secure_realloc(block: Option<MemoryBlock>, size: usize) -> Option<MemoryBlock> {
    require_memory("SecureRealloc", try_secure_realloc(block, size))
}

pub fn try_size_malloc(
    policy: MemoryPolicy,
    requested_size: usize,
) -> Result<MemoryBlock, MemoryError> {
    let effective_size = policy.effective_size(requested_size);
    let bucket_size = policy.bucket_size(requested_size);
    let allocation_size = bucket_size.unwrap_or(effective_size);

    let mut state = lock_state();
    state.stats.size_malloc_count = state.stats.size_malloc_count.saturating_add(1);
    state.stats.size_malloc_mem = state.stats.size_malloc_mem.saturating_add(effective_size);

    if let Some(bucket_size) = bucket_size {
        if policy == MemoryPolicy::NewAligned
            && effective_size < MEM_CHUNKLIMIT
            && !state.free_lists.contains_key(&(policy, bucket_size))
        {
            let mem_index = bucket_size / MEM_ALIGN;
            state.add_newmem_chunk(mem_index)?;
        }

        if let Some(block) = state
            .free_lists
            .get_mut(&(policy, bucket_size))
            .and_then(Vec::pop)
        {
            return Ok(MemoryBlock {
                requested_size,
                allocation_size: block.allocation_size,
                bytes: block.bytes,
            });
        }
    }

    state.secure_malloc_block(requested_size, allocation_size)
}

#[must_use]
pub fn size_malloc(policy: MemoryPolicy, requested_size: usize) -> MemoryBlock {
    require_memory("SizeMalloc", try_size_malloc(policy, requested_size))
}

pub fn size_free(policy: MemoryPolicy, block: MemoryBlock) {
    let effective_size = policy.effective_size(block.requested_size());
    let bucket_size = policy.bucket_size(block.requested_size());

    let mut state = lock_state();
    state.stats.size_free_count = state.stats.size_free_count.saturating_add(1);
    state.stats.size_free_mem = state.stats.size_free_mem.saturating_add(effective_size);

    if let Some(bucket_size) = bucket_size {
        state
            .free_lists
            .entry((policy, bucket_size))
            .or_default()
            .push(block);
    } else {
        state.stats.clb_free_count = state.stats.clb_free_count.saturating_add(1);
    }
}

pub fn try_mem_add_new_chunk(mem_index: usize) -> Result<usize, MemoryError> {
    lock_state().add_newmem_chunk(mem_index)
}

#[must_use]
pub fn mem_add_new_chunk(mem_index: usize) -> usize {
    require_memory("MemAddNewChunk", try_mem_add_new_chunk(mem_index))
}

pub fn try_secure_strdup(source: &str) -> Result<MemoryBlock, MemoryError> {
    let mut block = try_secure_malloc(source.len().saturating_add(1))?;
    block.allocation_bytes_mut()[..source.len()].copy_from_slice(source.as_bytes());
    Ok(block)
}

#[must_use]
pub fn secure_strdup(source: &str) -> MemoryBlock {
    require_memory("SecureStrdup", try_secure_strdup(source))
}

pub fn try_secure_strndup(source: &str, count: usize) -> Result<MemoryBlock, MemoryError> {
    let copy_len = source.len().min(count);
    let mut block = try_secure_malloc(copy_len.saturating_add(1))?;
    block.allocation_bytes_mut()[..copy_len].copy_from_slice(&source.as_bytes()[..copy_len]);
    Ok(block)
}

#[must_use]
pub fn secure_strndup(source: &str, count: usize) -> MemoryBlock {
    require_memory("SecureStrndup", try_secure_strndup(source, count))
}

pub fn try_int_array_alloc(size: usize) -> Result<Vec<i64>, MemoryError> {
    let Some(bytes) = size.checked_mul(mem::size_of::<i64>()) else {
        return Err(MemoryError::SizeOverflow {
            size,
            multiplier: mem::size_of::<i64>(),
        });
    };
    let _block = try_size_malloc(MemoryPolicy::OldExact, bytes)?;
    let mut values = Vec::new();
    if values.try_reserve_exact(size).is_err() {
        return Err(MemoryError::AllocationFailed { size: bytes });
    }
    values.resize(size, 0);
    Ok(values)
}

#[must_use]
pub fn int_array_alloc(size: usize) -> Vec<i64> {
    require_memory("IntArrayAlloc", try_int_array_alloc(size))
}

#[must_use]
pub fn mem_debug_print_stats() -> String {
    let stats = memory_stats();
    format!(
        "# Total SizeMalloc()ed memory: {} Bytes ({} requests)\n\
# Total SizeFree()ed   memory: {} Bytes ({} requests)\n\
# New requests: {:6} ({:6} by SecureMalloc(), {:6} by SecureRealloc())\n\
# Total SecureMalloc()ed memory: {} Bytes\n",
        stats.size_malloc_mem,
        stats.size_malloc_count,
        stats.size_free_mem,
        stats.size_free_count,
        stats
            .secure_malloc_count
            .saturating_add(stats.secure_realloc_count),
        stats.secure_malloc_count,
        stats.secure_realloc_count,
        stats.secure_malloc_mem
    )
}

#[must_use]
pub fn mem_free_list_print() -> String {
    let state = lock_state();
    let mut output = "# MemFreeListPrint()\n".to_owned();
    for ((policy, bucket_size), blocks) in &state.free_lists {
        if !blocks.is_empty()
            && writeln!(&mut output, "# {policy:?} {bucket_size}: {}", blocks.len()).is_err()
        {
            break;
        }
    }
    output
}

#[cfg(test)]
pub(crate) fn reset_memory_for_tests() {
    *lock_state() = MemoryState::default();
}

#[cfg(test)]
pub(crate) fn memory_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        int_array_alloc, mem_add_new_chunk, mem_debug_print_stats, mem_flush_free_list,
        mem_free_list_print, mem_is_low, memory_stats, memory_test_lock, reset_memory_for_tests,
        secure_malloc, secure_realloc, secure_strdup, secure_strndup, set_mem_is_low, size_free,
        size_malloc, try_int_array_alloc, try_mem_add_new_chunk, MemoryError, MemoryPolicy,
        MEM_ALIGN, MEM_MULTIPLIER,
    };

    #[test]
    fn old_policy_reuses_exact_size_blocks() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        let mut block = size_malloc(MemoryPolicy::OldExact, 64);
        block.allocation_bytes_mut()[0] = 7;
        size_free(MemoryPolicy::OldExact, block);

        let stats = memory_stats();
        assert_eq!(stats.free_list_blocks, 1);
        assert_eq!(stats.free_list_bytes, 64);

        let reused = size_malloc(MemoryPolicy::OldExact, 64);
        assert_eq!(reused.allocation_size(), 64);
        assert_eq!(reused.allocation_bytes()[0], 7);
        assert_eq!(mem_flush_free_list(), (0, 0));
    }

    #[test]
    fn new_policy_aligns_small_blocks_and_populates_chunks() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        let block = size_malloc(MemoryPolicy::NewAligned, 1);
        assert_eq!(block.allocation_size(), MEM_ALIGN);
        assert_eq!(block.requested_size(), 1);

        let stats = memory_stats();
        assert_eq!(stats.secure_malloc_count, 1);
        assert_eq!(stats.free_list_blocks, MEM_MULTIPLIER - 1);
        assert_eq!(stats.free_list_bytes, (MEM_MULTIPLIER - 1) * MEM_ALIGN);
    }

    #[test]
    fn explicit_newmem_chunk_adds_free_blocks() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        assert_eq!(mem_add_new_chunk(2), MEM_MULTIPLIER);
        let stats = memory_stats();
        assert_eq!(stats.free_list_blocks, MEM_MULTIPLIER);
        assert_eq!(stats.free_list_bytes, MEM_MULTIPLIER * MEM_ALIGN * 2);
    }

    #[test]
    fn try_newmem_chunk_reports_size_overflow() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        assert_eq!(
            try_mem_add_new_chunk(usize::MAX),
            Err(MemoryError::SizeOverflow {
                size: usize::MAX,
                multiplier: MEM_ALIGN
            })
        );
    }

    #[test]
    #[should_panic(expected = "MemAddNewChunk size overflow")]
    fn c_shaped_newmem_chunk_panics_on_size_overflow() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        let _ = mem_add_new_chunk(usize::MAX);
    }

    #[test]
    fn mem_is_low_accessors_preserve_c_global_shape() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        assert!(!mem_is_low());
        assert!(!set_mem_is_low(true));
        assert!(mem_is_low());
        assert!(set_mem_is_low(false));
        assert!(!mem_is_low());
    }

    #[test]
    fn secure_realloc_preserves_prefix_and_counts_shapes() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        let mut block = secure_malloc(3);
        block.allocation_bytes_mut().copy_from_slice(&[1, 2, 3]);
        let grown = secure_realloc(Some(block), 5).unwrap();
        assert_eq!(grown.allocation_bytes(), &[1, 2, 3, 0, 0]);

        assert_eq!(secure_realloc(Some(grown), 0), None);
        let allocated = secure_realloc(None, 2).unwrap();
        assert_eq!(allocated.allocation_size(), 2);

        let stats = memory_stats();
        assert_eq!(stats.secure_realloc_count, 3);
        assert_eq!(stats.secure_realloc_f_count, 1);
        assert_eq!(stats.secure_realloc_m_count, 1);
    }

    #[test]
    fn secure_string_helpers_copy_bytes_and_add_nul() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        assert_eq!(secure_strdup("abc").allocation_bytes(), b"abc\0");
        assert_eq!(secure_strndup("abcdef", 3).allocation_bytes(), b"abc\0");
        assert_eq!(secure_strndup("abc", 9).allocation_bytes(), b"abc\0");
    }

    #[test]
    fn int_arrays_are_zero_initialized_and_stats_are_printable() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        assert_eq!(int_array_alloc(4), vec![0; 4]);
        let stats_output = mem_debug_print_stats();
        assert!(stats_output.contains("Total SizeMalloc()ed memory"));
        assert!(mem_free_list_print().contains("MemFreeListPrint"));
    }

    #[test]
    fn try_int_array_alloc_reports_size_overflow() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        assert_eq!(
            try_int_array_alloc(usize::MAX),
            Err(MemoryError::SizeOverflow {
                size: usize::MAX,
                multiplier: std::mem::size_of::<i64>()
            })
        );
    }

    #[test]
    #[should_panic(expected = "IntArrayAlloc size overflow")]
    fn c_shaped_int_array_alloc_panics_on_size_overflow() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        let _ = int_array_alloc(usize::MAX);
    }
}
