set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-paramod-positions.txt
set logging overwrite on
set logging redirect on
set logging enabled on

set $first = -9223372036854772729
set $last = -9223372036854772727

# ParamodInfoCell fields are pointer-sized through new_orig/from, followed by
# CompactPos and ClausePos* pairs. global_clause_counter is the last identity
# assigned, so +1 is the identity ClauseSimParamodConstruct will allocate.
break ClauseSimParamodConstruct
commands
  silent
  set $next = *(long *) &global_clause_counter + 1
  if $next >= $first && $next <= $last
    set $info = (char *) $rdi
    set $new_orig = *(char **) ($info + 24)
    set $from = *(char **) ($info + 32)
    set $into = *(char **) ($info + 56)
    set $into_pos = *(char **) ($info + 72)
    set $literal = *(char **) ($into_pos + 8)
    if *(int *) ($into_pos + 16) == 2
      set $into_term = *(char **) ($literal + 16)
    else
      set $into_term = *(char **) ($literal + 8)
    end
    printf "next_ident=%ld new_orig=%ld from_parent=%ld from_cpos=%ld into_parent=%ld into_cpos=%ld into_term=%p entry_no=%ld f_code=%ld\n", $next, *(long *) $new_orig, *(long *) $from, *(long *) ($info + 40), *(long *) $into, *(long *) ($info + 64), $into_term, *(long *) ($into_term + 24), *(long *) $into_term
  end
  continue
end

break *ClauseAlloc+292
commands
  silent
  set $ident = *(long *) $rax
  if $ident == $last
    quit
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
