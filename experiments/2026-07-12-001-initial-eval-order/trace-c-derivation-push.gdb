set pagination off
set confirm off
set debuginfod enabled off
set $trace_count = 0

break ClausePushDerivation
commands
  silent
  set $trace_count = $trace_count + 1
  set $clause = (char *) $rdi
  printf "PUSH %ld clause=%p ident=%ld op=%ld ", $trace_count, $clause, *(long *) $clause, $rsi
  call (void) ClausePrintDBG((void *) stdout, (void *) $clause)
  call (int) fflush((void *) stdout)
  if $trace_count >= 160
    quit
  end
  continue
end

run --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p
