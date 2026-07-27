set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

set $rule_left = 0
set $rule_right = 0
set $tracking680 = 0
set $awaiting_compare = 0

# Capture the selected equation's normalized sides.
break *ClauseSetIndexedInsert+24
commands
  silent
  set $inserted = (char *) $r12
  if *(long *) $inserted == 2574
    set $literal = *(char **) ($inserted + 24)
    set $rule_left = *(char **) ($literal + 8)
    set $rule_right = *(char **) ($literal + 16)
    printf "rule left=%p right=%p\n", $rule_left, $rule_right
    set $left_arg0 = *(char **) ($rule_left + 104)
    set $left_arg1 = *(char **) ($rule_left + 112)
    set $left_arg10 = *(char **) ($left_arg1 + 104)
    set $left_arg11 = *(char **) ($left_arg1 + 112)
    set $right_arg0 = *(char **) ($rule_right + 104)
    set $right_arg1 = *(char **) ($rule_right + 112)
    set $right_arg10 = *(char **) ($right_arg1 + 104)
    set $right_arg11 = *(char **) ($right_arg1 + 112)
    printf "rule vars left=[%ld,%ld,%ld] right=[%ld,%ld,%ld]\n", *(long *) $left_arg0, *(long *) $left_arg10, *(long *) $left_arg11, *(long *) $right_arg0, *(long *) $right_arg10, *(long *) $right_arg11
  end
  continue
end

# SysV arguments are state, control, clause, and contraction options.
break ForwardContractClause
commands
  silent
  if *(long *) $rdx == 680
    set $tracking680 = 1
    printf "selected clause=680\n"
  end
  continue
end

# SysV arguments are ocb, bank, lside, rside, subst.
break instance_is_rule
commands
  silent
  if $tracking680 != 0 && $rule_left != 0 && (((char *) $rdx == $rule_left && (char *) $rcx == $rule_right) || ((char *) $rdx == $rule_right && (char *) $rcx == $rule_left))
    set $awaiting_compare = 1
    set $subst = (char *) $r8
    set $current = *(long *) ($subst + 8)
    set $stack = *(char ***) ($subst + 16)
    printf "instance direction=%s current=%ld\n", ((char *) $rdx == $rule_left ? "left" : "right"), $current
    set $i = 0
    while $i < $current
      set $var = $stack[$i]
      set $binding = *(char **) ($var + 16)
      printf "  var_f=%ld binding_f=%ld binding_arity=%d\n", *(long *) $var, *(long *) $binding, *(int *) ($binding + 12)
      set $i = $i + 1
    end
  end
  continue
end

# KBO6Greater+85 has the kbolincmp CompareResult in eax.
break *KBO6Greater+85
commands
  silent
  if $awaiting_compare != 0
    printf "KBO6 result=%d wb=%ld pos=%ld neg=%ld max_var=%ld\n", $eax, *(long *) ($rbp + 96), *(long *) ($rbp + 104), *(long *) ($rbp + 112), *(long *) ($rbp + 120)
    set $awaiting_compare = 0
  end
  continue
end

break TermAddRWLink
commands
  silent
  if $tracking680 != 0 && $rdx != 0 && *(long *) $rdx == 2574
    printf "linked source=%p replacement=%p\n", $rdi, $rsi
    quit
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
