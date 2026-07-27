set pagination off
set breakpoint pending on

break FormulaSetCNF2
commands
  silent
  set $tb = (char *)$rcx
  printf "phase=cnf-entry in_count=%lu insertions=%llu\n", *(unsigned long *)$tb, *(unsigned long long *)($tb + 8)
  continue
end

break ProofStateClausalPreproc
commands
  silent
  set $tb = *(char **)((char *)$rdi + 24)
  printf "phase=clausal-preproc-entry in_count=%lu insertions=%llu\n", *(unsigned long *)$tb, *(unsigned long long *)($tb + 8)
  continue
end

break ProofControlInit
commands
  silent
  set $tb = *(char **)((char *)$rdi + 24)
  printf "phase=proof-control-entry in_count=%lu insertions=%llu\n", *(unsigned long *)$tb, *(unsigned long long *)($tb + 8)
  continue
end

break ProofStateInit
commands
  silent
  set $tb = *(char **)((char *)$rdi + 24)
  printf "phase=proof-state-init-entry in_count=%lu insertions=%llu\n", *(unsigned long *)$tb, *(unsigned long long *)($tb + 8)
  continue
end

break Saturate
commands
  silent
  set $tb = *(char **)((char *)$rdi + 24)
  printf "phase=saturate-entry in_count=%lu insertions=%llu\n", *(unsigned long *)$tb, *(unsigned long long *)($tb + 8)
  continue
end

run
