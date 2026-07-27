set pagination off
set confirm off
set debuginfod enabled off
set $trace_count = 0

break ClauseDerivFindFirst
commands
  silent
  set $trace_count = $trace_count + 1
  set $clause = (char *) $rdi
  set $deriv = *(char **) ($clause + 0x48)
  printf "QUOTE %ld clause=%p ident=%ld deriv=%p", $trace_count, $clause, *(long *) $clause, $deriv
  if $deriv != 0
    set $sp = *(long *) ($deriv + 8)
    set $entries = *(long **) ($deriv + 16)
    printf " sp=%ld entries=", $sp
    set $pos = 0
    while $pos < $sp
      printf "%ld,", *($entries + $pos)
      set $pos = $pos + 1
    end
  end
  printf "\n"
  call (void) ClausePrintDBG((void *) stdout, (void *) $clause)
  call (int) fflush((void *) stdout)
  if $trace_count >= 120
    quit
  end
  continue
end

run --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p
