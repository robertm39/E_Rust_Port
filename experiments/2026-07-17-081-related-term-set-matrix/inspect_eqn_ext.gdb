set pagination off
set breakpoint pending on
break ccl_eqn.c:2860
commands
  silent
  printf "C EqnTermExtWeight props=%lu lcode=%ld rcode=%ld maxterm=%.9g result=%.9g\n", eq->properties, eq->lterm->f_code, eq->rterm->f_code, twe->max_term_multiplier, res
  continue
end
run
