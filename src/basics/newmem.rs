use crate::basics::memory::{
    self, MemoryBlock, MemoryError, MemoryPolicy, MemoryStats, MEM_ALIGN, MEM_ARR_SIZE,
    MEM_CHUNKLIMIT, MEM_FREE_PATTERN, MEM_MULTIPLIER, MEM_RSET_PATTERN,
};

pub use crate::basics::memory::mem_arr_min_index;

pub fn try_size_malloc_real(size: usize) -> Result<MemoryBlock, MemoryError> {
    memory::try_size_malloc(MemoryPolicy::NewAligned, size)
}

#[must_use]
pub fn size_malloc_real(size: usize) -> MemoryBlock {
    memory::size_malloc(MemoryPolicy::NewAligned, size)
}

pub fn size_free_real(block: MemoryBlock) {
    memory::size_free(MemoryPolicy::NewAligned, block);
}

pub fn try_mem_add_new_chunk(mem_index: usize) -> Result<usize, MemoryError> {
    memory::try_mem_add_new_chunk(mem_index)
}

#[must_use]
pub fn mem_add_new_chunk(mem_index: usize) -> usize {
    memory::mem_add_new_chunk(mem_index)
}

#[must_use]
pub fn mem_flush_free_list() -> (usize, usize) {
    (0, 0)
}

#[must_use]
pub fn secure_malloc(size: usize) -> MemoryBlock {
    memory::secure_malloc(size)
}

pub fn try_secure_malloc(size: usize) -> Result<MemoryBlock, MemoryError> {
    memory::try_secure_malloc(size)
}

#[must_use]
pub fn secure_realloc(block: Option<MemoryBlock>, size: usize) -> Option<MemoryBlock> {
    memory::secure_realloc(block, size)
}

pub fn try_secure_realloc(
    block: Option<MemoryBlock>,
    size: usize,
) -> Result<Option<MemoryBlock>, MemoryError> {
    memory::try_secure_realloc(block, size)
}

#[must_use]
pub fn secure_strdup(source: &str) -> MemoryBlock {
    memory::secure_strdup(source)
}

pub fn try_secure_strdup(source: &str) -> Result<MemoryBlock, MemoryError> {
    memory::try_secure_strdup(source)
}

#[must_use]
pub fn secure_strndup(source: &str, count: usize) -> MemoryBlock {
    memory::secure_strndup(source, count)
}

pub fn try_secure_strndup(source: &str, count: usize) -> Result<MemoryBlock, MemoryError> {
    memory::try_secure_strndup(source, count)
}

#[must_use]
pub fn int_array_alloc(size: usize) -> Vec<i64> {
    memory::int_array_alloc(size)
}

pub fn try_int_array_alloc(size: usize) -> Result<Vec<i64>, MemoryError> {
    memory::try_int_array_alloc(size)
}

#[must_use]
pub fn memory_stats() -> MemoryStats {
    memory::memory_stats()
}

#[must_use]
pub const fn constants() -> (usize, usize, usize, usize, u64, u64) {
    (
        MEM_ARR_SIZE,
        MEM_ALIGN,
        MEM_CHUNKLIMIT,
        MEM_MULTIPLIER,
        MEM_FREE_PATTERN,
        MEM_RSET_PATTERN,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        constants, mem_flush_free_list, memory_stats, size_free_real, size_malloc_real, MEM_ALIGN,
    };
    use crate::basics::memory::{memory_test_lock, reset_memory_for_tests};

    #[test]
    fn newmem_wrapper_uses_aligned_chunk_policy() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        let block = size_malloc_real(17);
        assert_eq!(block.allocation_size(), MEM_ALIGN * 2);
        size_free_real(block);

        let stats = memory_stats();
        assert_eq!(stats.size_malloc_count, 1);
        assert_eq!(stats.size_free_count, 1);
        assert!(stats.free_list_blocks >= 1);
    }

    #[test]
    fn newmem_flush_is_a_no_op() {
        let _guard = memory_test_lock();
        reset_memory_for_tests();

        let block = size_malloc_real(17);
        size_free_real(block);
        let before = memory_stats();
        assert!(before.free_list_blocks >= 1);

        assert_eq!(mem_flush_free_list(), (0, 0));
        let after = memory_stats();
        assert_eq!(after.free_list_blocks, before.free_list_blocks);
        assert_eq!(after.flush_count, before.flush_count);
    }

    #[test]
    fn exported_constants_match_c_header_values() {
        let (arr_size, align, chunk_limit, multiplier, free_pattern, reset_pattern) = constants();
        assert_eq!(arr_size, 8192);
        assert_eq!(align, 16);
        assert_eq!(chunk_limit, 256);
        assert_eq!(multiplier, 1024);
        assert_eq!(free_pattern, 0xFAFB_FAFA);
        assert_eq!(reset_pattern, 0);
    }
}
