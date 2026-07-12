set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

# At ClauseSetIndexedInsert+24 the optimized prologue has loaded the clause into
# r12. This build enables CLAUSE_PERM_IDENT, so ClauseCell.literals is at +24.
break *ClauseSetIndexedInsert+24
commands
  silent
  set $clause = (char *) $r12
  set $ident = *(long *) $clause
  if $ident == 2574
    set $literal = *(char **) ($clause + 24)
    set $props = *(unsigned int *) $literal
    printf "ident=%ld eqn_props=%ld oriented=%ld\n", $ident, $props, (($props & 16) != 0)
    quit
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
