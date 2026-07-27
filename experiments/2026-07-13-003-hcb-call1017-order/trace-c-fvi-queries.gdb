set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $queries = 0
set $captured = 0
set $max_captured = 200

# The optimized wrapper tail-calls this feature-zero specialization. Recursive
# levels call the unspecialized helper, so this breakpoint sees one hit per
# indexed subsumer query.
break 'clause_set_subsumes_clause_indexed.constprop.0'
commands
  silent
  set $queries = $queries + 1
  set $query = (char *) $rsi
  set $calls_before = *(long *) &ClauseClauseSubsumptionCalls
  continue
end

# All non-tail top-level returns join here. Result-register capture in this
# optimized build is not stable across the GDB inferior calls, so omit it.
break *('clause_set_subsumes_clause_indexed.constprop.0'+174)
commands
  silent
  set $query_clause = *(char **) ($query + 16)
  set $matcher_calls = *(long *) &ClauseClauseSubsumptionCalls - $calls_before
  if $matcher_calls > 0
    set $captured = $captured + 1
    if $captured == 1
      set $trace_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-fvi-queries.txt", "w")
    end
    call (int) fprintf($trace_file, "query=%ld raw_query=%ld ident=%ld matcher_calls=%ld clause=", $captured, $queries, *(long *) $query_clause, $matcher_calls)
    call (void) ClausePrintLOPFormat($trace_file, (void *) $query_clause, 1)
    call (int) fprintf($trace_file, " vector=")
    set $i = 0
    set $size = *(long *) $query
    set $array = *(char **) ($query + 8)
    while $i < $size
      call (int) fprintf($trace_file, "%ld,", *(long *) ($array + 8 * $i))
      set $i = $i + 1
    end
    call (int) fputc(10, $trace_file)
    if $captured == $max_captured
      call (int) fclose($trace_file)
      quit
    end
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
