set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $calls = 0
set $max_calls = 10000

# HCBStandardClauseSelect+99 follows the orphan loop in the optimized
# reference. Capture identifier-free LOP bodies so raw allocator-driven
# identifier permutations do not masquerade as structural divergence.
break *HCBStandardClauseSelect+99
commands
  silent
  set $calls = $calls + 1
  set $clause = (char *) $rbx
  if $calls == 1
    set $trace_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-hcb-structures.txt", "w")
  end
  call (int) fprintf($trace_file, "call=%ld ident=%ld clause=", $calls, *(long *) $clause)
  call (void) ClausePrintLOPFormat($trace_file, (void *) $clause, 1)
  call (int) fputc(10, $trace_file)
  if $calls == $max_calls
    call (int) fclose($trace_file)
    quit
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
