set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-clause-allocation-backtraces.txt
set logging overwrite on
set logging redirect on
set logging enabled on

set $first = -9223372036854772729
set $last = -9223372036854772727

# ClauseAlloc+292 has the fully initialized return ClauseCell* in RAX in the
# optimized reference. CLAUSE_PERM_IDENT puts the permanent allocation id at
# +0x8 in this build.
break *ClauseAlloc+292
commands
  silent
  set $clause = (char *) $rax
  set $ident = *(long *) $clause
  if $ident >= $first && $ident <= $last
    printf "allocated ident=%ld perm_ident=%ld\n", $ident, *(long *) ($clause + 8)
    backtrace 10
    set $body_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-clause-allocation-bodies.txt", "a")
    call (int) fprintf($body_file, "ident=%ld perm_ident=%ld clause=", $ident, *(long *) ($clause + 8))
    call (void) ClausePrint($body_file, (void *) $clause, 1)
    call (int) fputc(10, $body_file)
    call (int) fclose($body_file)
  end
  if $ident == $last
    quit
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
