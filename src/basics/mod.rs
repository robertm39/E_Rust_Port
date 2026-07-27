pub mod avlgeneric;
pub mod ddarrays;
pub mod defines;
pub mod dstacks;
pub mod dstrings;
pub mod error;
pub mod fixdarrays;
pub mod floattrees;
pub mod intmap;
pub mod memory;
pub mod min_heap;
pub mod newmem;
pub mod numtrees;
pub mod numxtrees;
pub mod objmaps;
pub mod objtrees;
pub mod os_wrapper;
pub mod partial_orderings;
pub mod pdarrays;
pub mod pdrangearrays;
pub mod perf_counters;
pub mod permastrings;
pub mod plist;
pub mod plocalstacks;
pub mod pqueue;
pub mod properties;
pub mod pstacks;
pub mod ptrees;
pub mod quadtrees;
pub mod regmem;
// Allowed measured-performance boundary: upstream E routes exact-size objects
// through a process-wide intrusive free list. The global allocator port keeps
// the required pointer manipulation private behind Rust's allocation API.
pub mod simple_stuff;
#[allow(unsafe_code)]
mod size_class_allocator;
pub mod stringtrees;
pub mod sysdate;
pub mod verbose;
