set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $calls = 0

break *HCBStandardClauseSelect+99
commands
  silent
  set $calls = $calls + 1
  if $calls == 1242
    set $capture_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-12-007-geo288-search/c-hidden-clause-3656.txt", "w")
    call (void) ClausePrint($capture_file, (void *) $rbx, 1)
    call (int) fputc(10, $capture_file)
    set $derivation = *(void **) ((char *) $rbx + 0x48)
    call (void) DerivationDebugPrint($capture_file, $derivation)
    call (int) fputc(10, $capture_file)
    call (int) fclose($capture_file)
    set $literal = *(void **) ((char *) $rbx + 0x18)
    set $literal_index = 0
    while $literal != 0
      set $left = *(void **) ((char *) $literal + 0x08)
      set $right = *(void **) ((char *) $literal + 0x10)
      printf "literal=%d props=%d left=%p left_entry=%ld right=%p right_entry=%ld\n", $literal_index, *(int *) $literal, $left, *(long *) ((char *) $left + 0x18), $right, *(long *) ((char *) $right + 0x18)
      set $literal = *(void **) ((char *) $literal + 0x20)
      set $literal_index = $literal_index + 1
    end
    set $trivial_result = (int) ClauseIsTrivial((void *) $rbx)
    printf "cached_neg=%d cached_pos=%d actual_literals=%d clause_is_trivial=%d\n", *(int *) ((char *) $rbx + 0x20), *(int *) ((char *) $rbx + 0x24), $literal_index, $trivial_result
    quit
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
