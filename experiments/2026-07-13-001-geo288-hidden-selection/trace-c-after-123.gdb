set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $occurrence = 0
set $remaining = 0
set $offset = 0

break ForwardContractClause
commands
  silent
  set $clause = (char *) $rdx
  set $ident = *(long *) $clause
  if $ident == -9223372036854775685
    set $occurrence = $occurrence + 1
    set $remaining = 6
    set $offset = 0
  end
  if $remaining > 0
    set $hcb = *(char **) ((char *) $rsi + 8)
    printf "occurrence=%ld offset=%ld ident=%ld current_eval=%d select_count=%ld\n", $occurrence, $offset, $ident, *(int *) ($hcb + 12), *(long *) ($hcb + 24)
    set $remaining = $remaining - 1
    set $offset = $offset + 1
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
