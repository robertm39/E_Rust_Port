set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

# The cached optimized reference has symbols but no line table. At +319 the
# processed-set and definition-store anchors have all been installed for the
# no-watchlist GEO288 run; rbp still owns the ProofState pointer.
break *fvi_param_init+319
commands
  silent
  set $state = (char *) $rbp
  set $set = *(char **) ($state + 0x70)
  set $anchor = *(char **) ($set + 0x28)
  set $perm = *(char **) ($anchor + 8)
  set $perm_size = *(long *) $perm
  printf "perm_size=%ld\n", $perm_size
  set $i = 0
  printf "perm="
  while $i < $perm_size
    printf "%ld ", *(long *) ($perm + 8 + 8 * $i)
    set $i = $i + 1
  end
  printf "\n"
  quit
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
