set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

set $target_left = 0
set $target_right = 0
set $tracking = 0

break *ClauseSetIndexedInsert+24
commands
  silent
  set $inserted = (char *) $r12
  if *(long *) $inserted == 680 || *(long *) $inserted == 712
    set $literal = *(char **) ($inserted + 24)
    set $left = *(char **) ($literal + 8)
    set $right = *(char **) ($literal + 16)
    printf "insert clause=%ld left=%p right=%p\n", *(long *) $inserted, $left, $right
    if *(long *) $inserted == 680
      set $target_left = $left
      set $target_right = $right
    end
    if *(long *) $inserted == 712
      set $left0 = *(char **) ($left + 104)
      set $left1 = *(char **) ($left + 112)
      set $left10 = *(char **) ($left1 + 104)
      set $left11 = *(char **) ($left1 + 112)
      set $right0 = *(char **) ($right + 104)
      set $right1 = *(char **) ($right + 112)
      set $right10 = *(char **) ($right1 + 104)
      set $right11 = *(char **) ($right1 + 112)
      printf "demod vars left=[%ld,%ld,%ld] right=[%ld,%ld,%ld]\n", *(long *) $left0, *(long *) $left10, *(long *) $left11, *(long *) $right0, *(long *) $right10, *(long *) $right11
    end
  end
  continue
end

break FindRewritableClausesIndexed
commands
  silent
  if *(long *) $rcx == 712
    set $tracking = 1
    printf "find demod=712 date=%ld\n", $r8
  end
  continue
end

break TermAddRWLink
commands
  silent
  if $tracking != 0 && $rdx != 0 && *(long *) $rdx == 712
    printf "link demod=712 source=%p replacement=%p result=%d\n", $rdi, $rsi, $r8d
  end
  continue
end

break *RemoveRewritableClausesIndexed+138
commands
  silent
  if $tracking != 0
    set $count = *(long *) ($r13 + 8)
    set $data = *(char ***) ($r13 + 16)
    printf "candidate count=%ld", $count
    set $i = 0
    while $i < $count
      printf " %ld", *(long *) $data[$i]
      set $i = $i + 1
    end
    printf "\n"
    printf "target clause=680 left=%p right=%p\n", $target_left, $target_right
    quit
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
