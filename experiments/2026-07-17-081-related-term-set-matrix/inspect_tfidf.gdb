set pagination off
set breakpoint pending on
break che_wfcb.c:110
commands
  silent
  printf "C eval ident=%ld score=%.9g\n", clause->ident, clause->evaluations->evals[pos].heuristic
  continue
end
run
