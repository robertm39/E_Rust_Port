set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

# Runtime generated-clause id LONG_MIN + 18945.
set $target = -9223372036854756863

# The optimized x86-64 reference has no C types/lines. HCBClauseEvaluate+169
# is its epilogue, r13 retains ClauseCell*, ClauseCell.evaluations is +0x38,
# and EvalCell's flexible array starts at +24 with a 32-byte cell stride.
break *HCBClauseEvaluate+169
commands
  silent
  set $clause = (char *) $r13
  set $ident = *(long *) $clause
  if $ident == $target
    set $eval = *(char **) ($clause + 0x38)
    set $eval_no = *(int *) $eval
    set $eval_count = *(long *) ($eval + 8)
    printf "evaluated ident=%ld eval=", $ident
    set $pos = 0
    while $pos < $eval_no
      printf "[%ld:%.10g:%ld]", *(long *) ($eval + 24 + ($pos * 32)), *(float *) ($eval + 32 + ($pos * 32)), $eval_count
      set $pos = $pos + 1
    end
    printf "\n"

    set $deriv = *(char **) ($clause + 0x48)
    if $deriv != 0
      set $current = *(long *) ($deriv + 8)
      set $values = *(char **) ($deriv + 16)
      printf "derivation current=%ld", $current
      set $index = 0
      while $index < $current
        printf " [%ld]=%#lx", $index, *(unsigned long *) ($values + 8 * $index)
        set $index = $index + 1
      end
      printf "\n"
    end
  end
  continue
end

# HCBStandardClauseSelect+99 follows the orphan loop on the canonical
# reference; rbx is the selected ClauseCell* and rbp is the HCBCell*. The
# active evaluation position is the int at HCBCell+12, before schedule advance.
break *HCBStandardClauseSelect+99
commands
  silent
  set $clause = (char *) $rbx
  if $clause != 0 && *(long *) $clause == $target
    printf "selected ident=%ld current_eval=%d select_count=%ld\n", *(long *) $clause, *(int *) ((char *) $rbp + 12), *(long *) ((char *) $rbp + 24)
    quit
  end
  continue
end

run --auto --output-level=1 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p
