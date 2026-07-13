set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

# Initial clause ids i_0_3608 through i_0_3610 are LONG_MIN + input id.
set $first = -9223372036854772200
set $last = -9223372036854772198
set $clause30 = -9223372036854775778

# Canonical optimized x86-64 layout, documented in experiment 006.
break *HCBClauseEvaluate+169
commands
  silent
  set $clause = (char *) $r13
  set $ident = *(long *) $clause
  if ($ident >= $first && $ident <= $last) || $ident == $clause30
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
  end
  continue
end

break *HCBStandardClauseSelect+99
commands
  silent
  set $clause = (char *) $rbx
  if $clause != 0
    set $ident = *(long *) $clause
    if ($ident >= $first && $ident <= $last) || $ident == $clause30
      printf "selected ident=%ld current_eval=%d select_count=%ld\n", $ident, *(int *) ((char *) $rbp + 12), *(long *) ((char *) $rbp + 24)
    end
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
