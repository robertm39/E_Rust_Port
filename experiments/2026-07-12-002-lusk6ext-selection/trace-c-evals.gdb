set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $trace_count = 0

# The optimized reference has no C types/lines. HCBClauseEvaluate+169 is its
# epilogue, ClauseCell.evaluations is at +0x38, and EvalCell's flexible array
# starts at +24 with a 32-byte SimpleEvalCell stride on this x86-64 build.
break *HCBClauseEvaluate+169
commands
  silent
  set $trace_count = $trace_count + 1
  set $clause = (char *) $r13
  set $ident = *(long *) $clause
  if $ident >= 60 && $ident <= 100
    set $eval = *(char **) ($clause + 0x38)
    set $eval_no = *(int *) $eval
    printf "TRACE call=%ld ident=%ld count=%ld eval=", $trace_count, $ident, *(long *) ($eval + 8)
    set $pos = 0
    while $pos < $eval_no
      printf "[%ld:%.10g]", *(long *) ($eval + 24 + ($pos * 32)), *(float *) ($eval + 32 + ($pos * 32))
      set $pos = $pos + 1
    end
    printf "\n"
  end
  if $trace_count >= 1000
    quit
  end
  continue
end

run --auto --output-level=6 --processed-clauses-limit=40 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
