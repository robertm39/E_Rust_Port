set pagination off
set confirm off
set debuginfod enabled off
set $trace_count = 0

# The cached optimized binary retains function symbols but not C types/lines.
# HCBClauseEvaluate+169 is its single epilogue. ClauseCell.evaluations is at
# +0x38, and EvalCell's flexible SimpleEvalCell array starts at +24 with a
# 32-byte stride in this x86-64 build.
break *HCBClauseEvaluate+169
commands
  silent
  set $trace_count = $trace_count + 1
  set $clause = (char *) $r13
  set $eval = *(char **) ($clause + 0x38)
  set $eval_no = *(int *) $eval
  printf "TRACE %ld ident=%ld count=%ld eval=", $trace_count, *(long *) $clause, *(long *) ($eval + 8)
  set $pos = 0
  while $pos < $eval_no
    printf "[%ld:%.10g]", *(long *) ($eval + 24 + ($pos * 32)), *(float *) ($eval + 32 + ($pos * 32))
    set $pos = $pos + 1
  end
  printf "\n"
  if $trace_count >= 80
    quit
  end
  continue
end

run --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p
