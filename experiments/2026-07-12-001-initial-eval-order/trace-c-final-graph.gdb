set pagination off
set confirm off
set debuginfod enabled off

break DerivationPrintConditional
commands
  silent
  set $derivation = (char *) $rdx
  set $ordered = *(char **) ($derivation + 32)
  set $count = *(long *) ($ordered + 8)
  set $nodes = *(char ***) ($ordered + 16)
  printf "FINAL_GRAPH nodes=%ld\n", $count
  set $index = 0
  while $index < $count
    set $node = $nodes[$index]
    set $clause = *(char **) ($node + 16)
    if $clause != 0
      set $deriv = *(char **) ($clause + 0x48)
      printf "NODE %ld clause=%p ident=%ld deriv=%p", $index, $clause, *(long *) $clause, $deriv
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
    end
    set $index = $index + 1
  end
  quit
end

run --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p
