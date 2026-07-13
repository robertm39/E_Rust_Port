set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $calls = 0

break *HCBStandardClauseSelect+99
commands
  silent
  set $calls = $calls + 1
  if $calls == 995
    set $clause = (char *) $rbx
    set $capture_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-hcb-call995-clause.txt", "w")
    call (void) ClausePrint($capture_file, (void *) $clause, 1)
    call (int) fputc(10, $capture_file)
    call (int) fclose($capture_file)
    printf "captured call=%ld ident=%ld\n", $calls, *(long *) $clause
    quit
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
