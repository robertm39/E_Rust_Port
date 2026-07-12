set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

# PDTreeInsert(tree, ClausePos*) receives the position in rsi on x86-64
# System V. ClausePosCell.clause is its first field and ClauseCell.ident is
# the clause's first field in this optimized reference build.
break PDTreeInsert
commands
  silent
  set $pos = (char *) $rsi
  set $clause = *(char **) $pos
  set $ident = *(long *) $clause
  if $ident == 571 || $ident == 2574
    printf "insert ident=%ld pos=%p side=%d\n", $ident, $pos, *(int *) ($pos + 16)
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
