use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::spin_loop;
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

const FREE_LIST_LIMIT: usize = 8192;
const FREE_LIST_MINIMUM: usize = mem::size_of::<*mut u8>();
const CACHE_ALIGNMENT: usize = 16;

static FREE_LIST_LOCK: AtomicBool = AtomicBool::new(false);
static FREE_LISTS: [AtomicPtr<u8>; FREE_LIST_LIMIT] =
    [const { AtomicPtr::new(ptr::null_mut()) }; FREE_LIST_LIMIT];

thread_local! {
    static FALLIBLE_ALLOCATION_DEPTH: Cell<usize> = const { Cell::new(0) };
}

struct FreeListGuard;

impl Drop for FreeListGuard {
    fn drop(&mut self) {
        FREE_LIST_LOCK.store(false, Ordering::Release);
    }
}

fn lock_free_lists() -> FreeListGuard {
    while FREE_LIST_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        spin_loop();
    }
    FreeListGuard
}

fn cacheable_size(layout: Layout) -> Option<usize> {
    let size = layout.size();
    ((FREE_LIST_MINIMUM..FREE_LIST_LIMIT).contains(&size) && layout.align() <= CACHE_ALIGNMENT)
        .then_some(size)
}

fn cached_layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(size, CACHE_ALIGNMENT).ok()
}

struct FallibleAllocationGuard;

impl Drop for FallibleAllocationGuard {
    fn drop(&mut self) {
        let _ = FALLIBLE_ALLOCATION_DEPTH.try_with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

fn with_fallible_allocation<T>(operation: impl FnOnce() -> T) -> T {
    let marked = FALLIBLE_ALLOCATION_DEPTH
        .try_with(|depth| {
            depth.set(depth.get().saturating_add(1));
        })
        .is_ok();
    let _guard = marked.then_some(FallibleAllocationGuard);
    operation()
}

fn fallible_allocation_active() -> bool {
    FALLIBLE_ALLOCATION_DEPTH
        .try_with(|depth| depth.get() != 0)
        .unwrap_or(false)
}

pub(crate) fn try_reserve_vec<T>(values: &mut Vec<T>, additional: usize) -> bool {
    with_fallible_allocation(|| values.try_reserve(additional).is_ok())
}

pub(crate) fn try_reserve_exact_vec<T>(values: &mut Vec<T>, additional: usize) -> bool {
    with_fallible_allocation(|| values.try_reserve_exact(additional).is_ok())
}

#[expect(
    clippy::cast_ptr_alignment,
    reason = "cacheable System blocks are allocated with 16-byte alignment"
)]
fn flush_free_lists() {
    let _guard = lock_free_lists();
    for (size, free_list) in FREE_LISTS.iter().enumerate().skip(FREE_LIST_MINIMUM) {
        let Some(layout) = cached_layout(size) else {
            continue;
        };
        let mut block = free_list.swap(ptr::null_mut(), Ordering::Relaxed);
        while !block.is_null() {
            // SAFETY: cached blocks are at least pointer-sized and aligned to
            // CACHE_ALIGNMENT. Deallocation initialized the first pointer-
            // sized slot with the next link, and the global lock excludes a
            // concurrent pop or push while that slot is read.
            let next = unsafe { block.cast::<*mut u8>().read() };
            // SAFETY: every block in this exact-size list came from
            // System::alloc with this reconstructed size and
            // CACHE_ALIGNMENT. The list owns the block exclusively, and it is
            // removed before being returned to System.
            unsafe { System.dealloc(block, layout) };
            block = next;
        }
    }
}

struct ESizeClassAllocator;

// SAFETY: successful allocations return uniquely owned System blocks or
// uniquely popped cached blocks that satisfy the requested size and alignment.
// Cacheable requests are allocated with CACHE_ALIGNMENT, which is at least the
// caller's requested alignment. Deallocation deterministically selects the
// same exact-size class from the original Layout. The global lock serializes
// every intrusive-link read and write, so a cached block cannot be aliased
// across threads. Non-cacheable layouts are passed unchanged to System.
unsafe impl GlobalAlloc for ESizeClassAllocator {
    /// # Safety
    ///
    /// The caller must uphold [`GlobalAlloc::alloc`]'s contract for `layout`
    /// and must manage a successful returned allocation through the same
    /// global allocator.
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "cacheable System blocks are allocated with 16-byte alignment"
    )]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Some(size) = cacheable_size(layout) {
            let _guard = lock_free_lists();
            let block = FREE_LISTS[size].load(Ordering::Relaxed);
            if !block.is_null() {
                // SAFETY: the exact-size list owns this pointer exclusively,
                // the block is pointer-sized and CACHE_ALIGNMENT-aligned, and
                // dealloc initialized its first slot with the next list link.
                let next = unsafe { block.cast::<*mut u8>().read() };
                FREE_LISTS[size].store(next, Ordering::Relaxed);
                return block;
            }
        }

        let system_layout = cacheable_size(layout)
            .and_then(cached_layout)
            .unwrap_or(layout);
        // SAFETY: system_layout is a valid Layout and System is the backing
        // allocator for every cache miss.
        let block = unsafe { System.alloc(system_layout) };
        if !block.is_null() {
            return block;
        }

        flush_free_lists();
        // SAFETY: the first allocation failure does not consume or alter the
        // requested Layout. After cached blocks are returned to System, the
        // same valid allocation may be retried exactly once.
        let block = unsafe { System.alloc(system_layout) };
        if block.is_null() {
            #[cfg(target_os = "linux")]
            if !fallible_allocation_active() {
                crate::inout::signals::finalize_memory_allocation_failure();
            }
        }
        block
    }

    /// # Safety
    ///
    /// `block` must identify a currently live allocation returned by this
    /// allocator for exactly `layout`, and the caller must relinquish all
    /// access to it before this call.
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "cacheable System blocks are allocated with 16-byte alignment"
    )]
    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        if let Some(size) = cacheable_size(layout) {
            let _guard = lock_free_lists();
            let next = FREE_LISTS[size].load(Ordering::Relaxed);
            // SAFETY: GlobalAlloc's contract gives this call unique ownership
            // of a live block allocated for the same Layout. Cacheable blocks
            // were allocated with CACHE_ALIGNMENT and have at least one
            // pointer-sized slot, so writing the private next link is aligned
            // and remains within the allocation.
            unsafe { block.cast::<*mut u8>().write(next) };
            FREE_LISTS[size].store(block, Ordering::Relaxed);
        } else {
            // SAFETY: non-cacheable allocations are returned directly by
            // System::alloc for this unchanged Layout, and GlobalAlloc's
            // contract supplies the original uniquely owned pointer.
            unsafe { System.dealloc(block, layout) };
        }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: ESizeClassAllocator = ESizeClassAllocator;

#[cfg(test)]
mod tests {
    use super::{
        cacheable_size, cached_layout, CACHE_ALIGNMENT, FREE_LIST_LIMIT, FREE_LIST_MINIMUM,
    };
    use std::alloc::Layout;
    #[cfg(target_os = "linux")]
    use std::io::Write;
    #[cfg(target_os = "linux")]
    use std::process::Command;
    use std::thread;

    #[test]
    fn cache_classes_match_the_c_exact_size_boundary() {
        assert_eq!(
            cacheable_size(Layout::from_size_align(FREE_LIST_MINIMUM - 1, 1).unwrap()),
            None
        );
        assert_eq!(
            cacheable_size(Layout::from_size_align(FREE_LIST_MINIMUM, 1).unwrap()),
            Some(FREE_LIST_MINIMUM)
        );
        assert_eq!(
            cacheable_size(Layout::from_size_align(FREE_LIST_LIMIT - 1, 8).unwrap()),
            Some(FREE_LIST_LIMIT - 1)
        );
        assert_eq!(
            cacheable_size(Layout::from_size_align(FREE_LIST_LIMIT, 8).unwrap()),
            None
        );
        assert_eq!(
            cacheable_size(Layout::from_size_align(64, CACHE_ALIGNMENT * 2).unwrap()),
            None
        );
    }

    #[test]
    fn cached_layout_preserves_exact_size_and_strengthens_alignment() {
        let layout = cached_layout(152).unwrap();
        assert_eq!(layout.size(), 152);
        assert_eq!(layout.align(), CACHE_ALIGNMENT);
    }

    #[test]
    fn global_cache_preserves_growth_and_high_alignment() {
        #[repr(align(64))]
        struct Aligned([u8; 64]);

        let aligned = Box::new(Aligned([37; 64]));
        assert_eq!(aligned.0, [37; 64]);
        assert_eq!(
            std::ptr::from_ref(&*aligned).addr() % std::mem::align_of::<Aligned>(),
            0
        );

        let mut bytes = Vec::with_capacity(9);
        bytes.extend(0_u8..9);
        for round in 0..32 {
            bytes.reserve_exact(17 + round);
            assert_eq!(&bytes[..9], &(0_u8..9).collect::<Vec<_>>());
        }
    }

    #[test]
    fn global_cache_serializes_parallel_reuse() {
        let workers = (0..4)
            .map(|worker| {
                thread::spawn(move || {
                    let worker_byte = u8::try_from(worker).unwrap();
                    for round in 0..2_000 {
                        let size = 8 + (round + worker * 17) % 1024;
                        let mut bytes = vec![worker_byte; size];
                        bytes.reserve_exact(size + 1);
                        bytes.resize(size * 2, round.to_le_bytes()[0]);
                        assert!(bytes[..size].iter().all(|byte| *byte == worker_byte));
                    }
                })
            })
            .collect::<Vec<_>>();

        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_allocation_failure_reports_resource_out_without_aborting() {
        const CHILD_ENV: &str = "UMLAUT_OOM_REPORT_CHILD";
        if std::env::var_os(CHILD_ENV).is_some() {
            crate::basics::error::init_error("umlaut-oom-test");
            assert_ne!(
                crate::basics::os_wrapper::set_memory_limit(64 * 1024 * 1024),
                crate::basics::os_wrapper::RLimResult::Failed
            );
            let mut fallible = Vec::<u8>::new();
            assert!(!super::try_reserve_exact_vec(
                &mut fallible,
                128 * 1024 * 1024
            ));
            std::io::stderr()
                .lock()
                .write_all(b"fallible-returned\n")
                .unwrap();
            let mut allocation = Vec::<u8>::with_capacity(128 * 1024 * 1024);
            allocation.resize(allocation.capacity(), 1);
            std::process::exit(99);
        }

        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "basics::size_class_allocator::tests::linux_allocation_failure_reports_resource_out_without_aborting",
                "--nocapture",
            ])
            .env(CHILD_ENV, "1")
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(2));
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stdout.contains(
                "%% Failure: Resource limit exceeded (memory)\n%% SZS status ResourceOut\n"
            ),
            "{stdout}"
        );
        assert_eq!(
            stderr,
            "fallible-returned\numlaut-oom-test: Out of Memory\n"
        );
    }
}
