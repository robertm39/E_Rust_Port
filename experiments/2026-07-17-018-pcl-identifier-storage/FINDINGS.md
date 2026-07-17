# PCL identifier storage and comparison audit

## Status

Completed for Bead `E_Rust_Port-j76.2.126`. Rust's live-component vector is an
evidence-backed replacement for C's sentinel-terminated integer `PDArray`, and
the protocol-facing comparison retains C's exact truncating result surface. The
vendored C source remained unchanged.

## Storage and parsing

C allocates two zero-filled `long` slots with a growth quantum of two. Parsing
writes fullstop-separated decimal components followed by
`NO_PCL_ID_ELEMENT == -1`; all printing and comparison loops trust that the
terminator exists.

Both scanners call a sequence of decimal digits `PosInt`, so component zero is
valid despite the name and source comments. Negative one cannot be parsed as a
component and is purely a storage sentinel.

Rust stores only live `i64` components. Vector length is the terminator, cannot
be mistaken for component zero, and cannot be absent or stale. An empty
allocated identifier has zero capacity and is explicitly rejected by printers;
valid input allocates geometrically. A 64-component regression round-trips the
plain spelling and confirms no sentinel is stored.

## Output

Plain output joins components with full stops. Formatted output gives only the
first component C's minimum width of seven. TSTP output leaves singletons as a
decimal integer and rewrites compound identifiers to
`pclid<first>_<second>...`. Zero components retain all of those shapes.

## Comparison

C compares the first differing `long` values by subtraction and returns the
result cast to `int`; it reads `-1` when one identifier ends. Rust injects that
same sentinel at vector exhaustion, uses wrapping `i64` subtraction to make the
C machine result defined, and truncates to `i32`.

The compatibility surface is not a strict ordering for all parsed values. On
the intended LP64-shaped path, comparing component `4294967296` with `0`
returns zero after truncation even though the identifiers differ. Regression
coverage pins this because full-protocol search/insertion currently uses the C
comparator. Replacing it with strict lexicographic ordering is post-compatibility
work, not a representation fix.

Related cleanup remains tracked by Beads `E_Rust_Port-j76.4.935` through `.937`
and `E_Rust_Port-j76.3.44`.

## Performance

Valid identifiers remain contiguous and require one allocation in either
representation. Rust avoids a stored terminator and direct iteration avoids
repeated dynamic-array element helpers. PCL identifiers are normally short;
there is no evidence that emulating C's exact two-slot capacity quantum would
improve proof parsing or output.

## Validation

- focused identifier tests cover empty allocation, zero components, 64-element
  growth, scanner advancement, exact plain/formatted/TSTP output, sentinel
  prefix ordering, wide-component truncation, and negative-input rejection;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
