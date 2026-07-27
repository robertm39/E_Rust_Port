set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-ident276-er-steps.txt
set logging overwrite on
set logging enabled on
set $track = 0
set $step = 0

break ForwardContractClause
commands
  silent
  if *(long *) $rdx == -9223372036854775532 && $track == 0
    set $track = 1
  end
  continue
end

break ClausePushDerivation
commands
  silent
  set $clause = (char *) $rdi
  if $track && *(long *) $clause == -9223372036854775532
    set $step = $step + 1
    printf "step=%ld\n", $step
    set $capture_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-ident276-er-steps-rendered.txt", $step == 1 ? "w" : "a")
    call (int) fprintf($capture_file, "step=%ld\n", $step)
    call (void) ClausePrint($capture_file, (void *) $clause, 1)
    call (int) fputc(10, $capture_file)
    set $literal = *(char **) ($clause + 0x18)
    set $literal_index = 0
    while $literal != 0
      set $left = *(char **) ($literal + 0x08)
      call (int) fprintf($capture_file, "literal=%d left_f_code=%d left_entry=%ld", $literal_index, *(int *) $left, *(long *) ($left + 0x18))
      if *(int *) ($left + 12) >= 1
        set $arg0 = *(char **) ($left + 0x68)
        call (int) fprintf($capture_file, " arg0_f_code=%d", *(int *) $arg0)
      end
      if *(int *) ($left + 12) >= 2
        set $arg1 = *(char **) ($left + 0x70)
        call (int) fprintf($capture_file, " arg1_f_code=%d", *(int *) $arg1)
      end
      call (int) fputc(10, $capture_file)
      set $literal = *(char **) ($literal + 0x20)
      set $literal_index = $literal_index + 1
    end
    call (int) fclose($capture_file)
    if $step == 3
      quit
    end
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
