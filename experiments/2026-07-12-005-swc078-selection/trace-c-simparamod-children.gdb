set pagination off
set confirm off
set debuginfod enabled off
set $first_children = 0
set $second_children = 0

# ClausePushDerivation(child, operation, parent1, parent2). DCSimParamod is
# 4633 in this build. Initial-clause identifiers encode i_0_N as LONG_MIN+N.
break ClausePushDerivation
commands
  silent
  if $rsi == 4633 && $rdx != 0 && $rcx != 0
    set $parent1 = *(long *) $rdx
    set $parent2 = *(long *) $rcx
    if (($parent1 == -9223372036854775459 && $parent2 == -9223372036854775287) || ($parent2 == -9223372036854775459 && $parent1 == -9223372036854775287))
      set $first_children = $first_children + 1
      printf "first-stage child=%ld ident=%ld parents=[%ld,%ld] ", $first_children, *(long *) $rdi, $parent1, $parent2
      call (void) ClausePrintDBG((void *) stderr, (void *) $rdi)
      call (int) fflush((void *) stderr)
    end
    if (($parent1 == -9223372036854775281 && $parent2 == -9223372036854775287) || ($parent2 == -9223372036854775281 && $parent1 == -9223372036854775287))
      set $second_children = $second_children + 1
      printf "second-stage child=%ld ident=%ld parents=[%ld,%ld] ", $second_children, *(long *) $rdi, $parent1, $parent2
      call (void) ClausePrintDBG((void *) stderr, (void *) $rdi)
      call (int) fflush((void *) stderr)
      quit
    end
  end
  continue
end

run --auto --output-level=1 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p > /dev/null
