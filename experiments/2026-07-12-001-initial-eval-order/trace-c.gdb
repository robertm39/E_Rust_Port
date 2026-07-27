set pagination off
set confirm off
set debuginfod enabled off
set $trace_count = 0

break HCBClauseEvaluate
commands
  silent
  set $trace_count = $trace_count + 1
  printf "TRACE %ld hcb=%p clause=%p\n", $trace_count, $rdi, $rsi
  call (void) ClausePrintDBG((void *) stdout, (void *) $rsi)
  if $trace_count >= 80
    quit
  end
  continue
end

run --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p
