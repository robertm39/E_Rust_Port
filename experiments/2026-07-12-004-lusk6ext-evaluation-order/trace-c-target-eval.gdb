set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $seen594 = 0
set $seen618 = 0

# The optimized reference has no C types/lines. HCBClauseEvaluate+169 is its
# epilogue, r13 retains the ClauseCell pointer, ClauseCell.evaluations is at
# +0x38, and EvalCell's flexible array starts at +24 with a 32-byte
# SimpleEvalCell stride on this x86-64 build.
break *HCBClauseEvaluate+169
commands
  silent
  set $clause = (char *) $r13
  set $ident = *(long *) $clause
  if $ident == 594 || $ident == 618
    set $eval = *(char **) ($clause + 0x38)
    set $eval_no = *(int *) $eval
    set $eval_count = *(long *) ($eval + 8)
    printf "target ident=%ld eval=", $ident
    set $pos = 0
    while $pos < $eval_no
      printf "[%ld:%.10g:%ld]", *(long *) ($eval + 24 + ($pos * 32)), *(float *) ($eval + 32 + ($pos * 32)), $eval_count
      set $pos = $pos + 1
    end
    printf "\n"
    if $ident == 594
      set $seen594 = 1
    end
    if $ident == 618
      set $seen618 = 1
    end
    if $seen594 != 0 && $seen618 != 0
      quit
    end
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
