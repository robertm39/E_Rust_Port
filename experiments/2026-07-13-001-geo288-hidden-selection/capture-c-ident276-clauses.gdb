set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-ident276-clauses.txt
set logging overwrite on
set logging enabled on
set $occurrence = 0

break ForwardContractClause
commands
  silent
  set $clause = (char *) $rdx
  if *(long *) $clause == -9223372036854775532
    set $occurrence = $occurrence + 1
    set $bank = *(char **) ((char *) $rdi + 24)
    printf "occurrence=%ld in_count=%lu\n", $occurrence, *(unsigned long *) $bank
    if $occurrence == 1
      set $capture_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-ident276-clauses-rendered.txt", "w")
    end
    call (int) fprintf($capture_file, "occurrence=%ld in_count=%lu\n", $occurrence, *(unsigned long *) $bank)
    call (void) ClausePrint($capture_file, (void *) $clause, 1)
    call (int) fputc(10, $capture_file)
    set $derivation = *(void **) ($clause + 0x48)
    call (void) DerivationDebugPrint($capture_file, $derivation)
    call (int) fputc(10, $capture_file)
    set $literal = *(char **) ($clause + 0x18)
    set $literal_index = 0
    while $literal != 0
      set $left = *(char **) ($literal + 0x08)
      call (int) fprintf($capture_file, "literal=%d props=%d left_f_code=%d left_entry=%ld", $literal_index, *(int *) $literal, *(int *) $left, *(long *) ($left + 0x18))
      if *(int *) ($left + 12) >= 1
        set $arg0 = *(char **) ($left + 0x68)
        call (int) fprintf($capture_file, " arg0_f_code=%d arg0_entry=%ld", *(int *) $arg0, *(long *) ($arg0 + 0x18))
      end
      if *(int *) ($left + 12) >= 2
        set $arg1 = *(char **) ($left + 0x70)
        call (int) fprintf($capture_file, " arg1_f_code=%d arg1_entry=%ld", *(int *) $arg1, *(long *) ($arg1 + 0x18))
      end
      call (int) fputc(10, $capture_file)
      set $literal = *(char **) ($literal + 0x20)
      set $literal_index = $literal_index + 1
    end
    if $occurrence == 2
      call (int) fclose($capture_file)
      quit
    end
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
