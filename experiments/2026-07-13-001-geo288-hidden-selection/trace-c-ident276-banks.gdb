set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-ident276-banks.txt
set logging overwrite on
set logging enabled on
set $selection = 0
set $occurrence = 0

break ForwardContractClause
commands
  silent
  set $selection = $selection + 1
  set $ident = *(long *) $rdx
  set $bank = *(char **) ((char *) $rdi + 24)
  set $in_count = *(unsigned long *) $bank
  if $ident == -9223372036854775532
    set $occurrence = $occurrence + 1
    printf "ident276 occurrence=%ld selection=%ld in_count=%lu\n", $occurrence, $selection, $in_count
  end
  if $in_count >= 45000 && $in_count <= 48000
    printf "window selection=%ld ident=%ld in_count=%lu\n", $selection, $ident, $in_count
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
