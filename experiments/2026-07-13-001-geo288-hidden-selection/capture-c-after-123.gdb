set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $occurrence = 0
set $capture_next = 0

break ForwardContractClause
commands
  silent
  set $clause = (char *) $rdx
  if $capture_next
    set $capture_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-selected-after-123.txt", "w")
    call (void) ClausePrint($capture_file, (void *) $clause, 1)
    call (int) fputc(10, $capture_file)
    set $derivation = *(void **) ($clause + 0x48)
    call (void) DerivationDebugPrint($capture_file, $derivation)
    call (int) fputc(10, $capture_file)
    call (int) fclose($capture_file)
    printf "captured ident=%ld after initial clause 123\n", *(long *) $clause
    quit
  end
  if *(long *) $clause == -9223372036854775685
    set $occurrence = $occurrence + 1
    if $occurrence == 3
      set $capture_next = 1
    end
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
