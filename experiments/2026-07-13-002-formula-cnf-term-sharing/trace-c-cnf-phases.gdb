set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set logging file /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/.artifacts/experiments/2026-07-13-002-formula-cnf-term-sharing/c-cnf-phases.txt
set logging overwrite on
set logging enabled on
set $formula = 0
set $gc = 0

break FormulaSetCNF2
commands
  silent
  printf "phase=cnf-entry in_count=%lu insertions=%llu recovered=%llu live=%ld\n", *(unsigned long *)$rcx, *(unsigned long long *)($rcx + 8), *(unsigned long long *)($rcx + 16), *(long *)($rcx + 96)
  continue
end

break FormulaSetSimplify
commands
  silent
  printf "phase=simplify-entry in_count=%lu insertions=%llu recovered=%llu live=%ld\n", *(unsigned long *)$rsi, *(unsigned long long *)($rsi + 8), *(unsigned long long *)($rsi + 16), *(long *)($rsi + 96)
  continue
end

break TFormulaSetIntroduceDefs
commands
  silent
  printf "phase=defs-entry in_count=%lu insertions=%llu recovered=%llu live=%ld\n", *(unsigned long *)$rdx, *(unsigned long long *)($rdx + 8), *(unsigned long long *)($rdx + 16), *(long *)($rdx + 96)
  continue
end

break WFormulaCNF2
commands
  silent
  set $formula = $formula + 1
  printf "phase=formula-entry formula=%ld in_count=%lu insertions=%llu recovered=%llu live=%ld\n", $formula, *(unsigned long *)$rdx, *(unsigned long long *)($rdx + 8), *(unsigned long long *)($rdx + 16), *(long *)($rdx + 96)
  continue
end

break TBGCCollect
commands
  silent
  set $gc = $gc + 1
  printf "phase=gc-entry gc=%ld in_count=%lu insertions=%llu recovered=%llu live=%ld\n", $gc, *(unsigned long *)$rdi, *(unsigned long long *)($rdi + 8), *(unsigned long long *)($rdi + 16), *(long *)($rdi + 96)
  continue
end

break ProofStateClausalPreproc
commands
  silent
  set $tb = *(char **)((char *)$rdi + 24)
  printf "phase=clausal-preproc-entry in_count=%lu insertions=%llu recovered=%llu live=%ld\n", *(unsigned long *)$tb, *(unsigned long long *)($tb + 8), *(unsigned long long *)($tb + 16), *(long *)($tb + 96)
  continue
end

run --auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
