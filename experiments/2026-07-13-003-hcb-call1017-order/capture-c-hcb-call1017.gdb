set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-hcb-call1017.txt
set logging overwrite on
set logging redirect on
set logging enabled on
set $calls = 0

# HCBStandardClauseSelect+99 follows the orphan loop in the optimized
# reference. RBX is the selected ClauseCell*, and RBP is the HCBCell*.
break *HCBStandardClauseSelect+99
commands
  silent
  set $calls = $calls + 1
  if $calls == 1017 || $calls == 1025
    set $clause = (char *) $rbx
    set $eval = *(char **) ($clause + 0x38)
    set $eval_no = *(int *) $eval
    set $eval_count = *(long *) ($eval + 8)
    printf "call=%ld ident=%ld date=%ld weight=%ld create_date=%ld proof_depth=%ld proof_size=%ld eval=", $calls, *(long *) $clause, *(long *) ($clause + 8), *(long *) ($clause + 0x30), *(long *) ($clause + 0x50), *(long *) ($clause + 0x58), *(long *) ($clause + 0x60)
    set $pos = 0
    while $pos < $eval_no
      printf "[%ld:%.10g:%ld]", *(long *) ($eval + 24 + ($pos * 32)), *(float *) ($eval + 32 + ($pos * 32)), $eval_count
      set $pos = $pos + 1
    end
    printf "\nderivation="
    set $deriv = *(char **) ($clause + 0x48)
    if $deriv == 0
      printf "none"
    else
      set $current = *(long *) ($deriv + 8)
      set $values = *(char **) ($deriv + 16)
      set $index = 0
      while $index < $current
        printf "[%ld]=%#lx ", $index, *(unsigned long *) ($values + 8 * $index)
        set $index = $index + 1
      end
    end
    if $calls == 1017
      set $clause_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-hcb-call1017-clause.txt", "w")
    else
      set $clause_file = (void *) fopen("/mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-hcb-call1025-clause.txt", "w")
    end
    call (void) ClausePrint($clause_file, (void *) $clause, 1)
    call (int) fputc(10, $clause_file)
    call (int) fclose($clause_file)
    printf "\n"
  end
  if $calls == 1025
    quit
  end
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
