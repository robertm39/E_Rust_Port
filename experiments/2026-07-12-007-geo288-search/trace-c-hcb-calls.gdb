set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-12-007-geo288-search/c-hcb-calls.txt
set logging overwrite on
set logging redirect on
set logging enabled on
set $calls = 0

break *HCBStandardClauseSelect+99
commands
  silent
  set $calls = $calls + 1
  set $clause = (char *) $rbx
  if $calls >= 1
    if $clause == 0
      printf "call=%ld ident=null current_eval=%d select_count=%ld\n", $calls, *(int *) ((char *) $rbp + 12), *(long *) ((char *) $rbp + 24)
    else
      printf "call=%ld ident=%ld current_eval=%d select_count=%ld\n", $calls, *(long *) $clause, *(int *) ((char *) $rbp + 12), *(long *) ((char *) $rbp + 24)
    end
  end
  if $calls == 1300
    quit
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
