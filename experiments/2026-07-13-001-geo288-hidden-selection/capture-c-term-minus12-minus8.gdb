set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-term-late-under-276.txt
set logging overwrite on
set logging enabled on
set $selected_ident = 0

break ForwardContractClause
commands
  silent
  set $selected_ident = *(long *) $rdx
  if $selected_ident == -9223372036854775532
    enable 2
  else
    disable 2
  end
  continue
end

break TermCellStoreInsert
condition 2 *(unsigned long *) ((char *) $rdi - 96) >= 47000 && *(int *) $rsi == 31 && *(int *) ((char *) $rsi + 12) == 2 && *(int *) *(char **) ((char *) $rsi + 0x68) == -12 && *(int *) *(char **) ((char *) $rsi + 0x70) == -8
commands
  silent
  set $candidate = (char *) $rsi
  set $arg0 = *(char **) ($candidate + 0x68)
  set $arg1 = *(char **) ($candidate + 0x70)
  set $bank = (char *) $rdi - 96
  printf "attempt selected=%ld in_count=%lu arg0_entry=%ld arg1_entry=%ld\n", $selected_ident, *(unsigned long *) $bank, *(long *) ($arg0 + 0x18), *(long *) ($arg1 + 0x18)
  quit
end
disable 2

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
