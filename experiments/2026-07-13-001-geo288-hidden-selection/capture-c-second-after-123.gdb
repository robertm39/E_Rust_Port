set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $occurrence = 0
set $capture_remaining = 0

break ForwardContractClause
commands
  silent
  set $clause = (char *) $rdx
  if $capture_remaining > 0
    set $capture_remaining = $capture_remaining - 1
    if $capture_remaining == 0
      set $capture_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-second-selected-after-123.txt", "w")
      call (void) ClausePrint($capture_file, (void *) $clause, 1)
      call (int) fputc(10, $capture_file)
      set $derivation = *(void **) ($clause + 0x48)
      call (void) DerivationDebugPrint($capture_file, $derivation)
      call (int) fputc(10, $capture_file)
      call (int) fclose($capture_file)
      set $literal = *(void **) ($clause + 0x18)
      set $literal_index = 0
      while $literal != 0
        set $left = *(void **) ((char *) $literal + 0x08)
        set $right = *(void **) ((char *) $literal + 0x10)
        printf "literal=%d props=%d left_entry=%ld right_entry=%ld f_code=%d arity=%d", $literal_index, *(int *) $literal, *(long *) ((char *) $left + 0x18), *(long *) ((char *) $right + 0x18), *(int *) $left, *(int *) ((char *) $left + 12)
        if $literal_index == 15 || $literal_index == 16
          set $arg0 = *(char **) ((char *) $left + 0x68)
          set $arg1 = *(char **) ((char *) $left + 0x70)
          printf " arg0_f_code=%d arg0_entry=%ld arg1_f_code=%d arg1_entry=%ld", *(int *) $arg0, *(long *) ($arg0 + 0x18), *(int *) $arg1, *(long *) ($arg1 + 0x18)
        end
        printf "\n"
        set $literal = *(void **) ((char *) $literal + 0x20)
        set $literal_index = $literal_index + 1
      end
      set $trivial_result = (int) ClauseIsTrivial((void *) $clause)
      printf "literal_count=%d clause_is_trivial=%d\n", $literal_index, $trivial_result
      printf "captured ident=%ld as second selection after initial clause 123\n", *(long *) $clause
      quit
    end
  end
  if *(long *) $clause == -9223372036854775685
    set $occurrence = $occurrence + 1
    if $occurrence == 3
      set $capture_remaining = 2
    end
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
