set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-ident417-insertions.txt
set logging overwrite on
set logging enabled on
set $occurrence = 0
set $active = 0

break ForwardContractClause
commands
  silent
  if $active
    quit
  end
  set $clause = (char *)$rdx
  if *(long *)$clause == -9223372036854775391
    set $occurrence = $occurrence + 1
    if $occurrence == 2
      set $bank = *(char **)((char *)$rdi + 24)
      printf "occurrence=2 in_count=%lu\n", *(unsigned long *)$bank
      set $capture_file = (void *)fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/c-ident417-clause.txt", "w")
      call (void)ClausePrint($capture_file, (void *)$clause, 1)
      call (int)fputc(10, $capture_file)
      call (int)fclose($capture_file)
      set $active = 1
      enable 2
    end
  end
  continue
end

break TermCellStoreInsert
condition 2 $active && *(unsigned long *)((char *)$rdi - 96) >= 47000
commands
  silent
  set $candidate = (char *)$rsi
  set $arity = *(int *)($candidate + 12)
  printf "attempt in_count=%lu f_code=%d arity=%d entry=%ld", *(unsigned long *)((char *)$rdi - 96), *(int *)$candidate, $arity, *(long *)($candidate + 0x18)
  if $arity >= 1
    set $arg0 = *(char **)($candidate + 0x68)
    printf " arg0_f_code=%d arg0_entry=%ld", *(int *)$arg0, *(long *)($arg0 + 0x18)
  end
  if $arity >= 2
    set $arg1 = *(char **)($candidate + 0x70)
    printf " arg1_f_code=%d arg1_entry=%ld", *(int *)$arg1, *(long *)($arg1 + 0x18)
  end
  printf "\n"
  if *(int *)$candidate == 148 && $arity == 1 && *(int *)$arg0 == -1
    bt 16
  end
  continue
end
disable 2

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
