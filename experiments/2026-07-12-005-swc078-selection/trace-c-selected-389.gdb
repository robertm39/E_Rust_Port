set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $selected = 0

# There are 214 documented presaturation selections. The first post-
# presaturation mismatch is ordinal 175, hence call 389 overall.
break *ProcessClause+2112
commands
  silent
  set $selected = $selected + 1
  if $selected % 50 == 0
    printf "selected checkpoint=%ld\n", $selected
  end
  if $selected == 389
    set $clause = (char *) $r14
    printf "selected=%ld ident=%ld\n", $selected, *(long *) $clause
    call (void) ClausePrintDBG((void *) stdout, (void *) $clause)
    quit
  end
  continue
end

run --auto --output-level=1 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p
