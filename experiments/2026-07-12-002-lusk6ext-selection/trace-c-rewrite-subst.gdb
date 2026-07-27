set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

set $rule_left = 0
set $rule_right = 0
set $rule_hits = 0
set $awaiting_instance_compare = 0
set $last_query = 0

python
import gdb
import struct

class DumpTerm(gdb.Command):
    def __init__(self):
        super().__init__("dump-term", gdb.COMMAND_DATA)

    def invoke(self, argument, from_tty):
        address = int(gdb.parse_and_eval(argument))
        inferior = gdb.selected_inferior()

        def read(address, size):
            return bytes(inferior.read_memory(address, size))

        def walk(term, depth, prefix):
            f_code = struct.unpack("q", read(term, 8))[0]
            arity = struct.unpack("i", read(term + 12, 4))[0]
            gdb.write(f"{prefix}{term:#x} f={f_code} arity={arity}\n")
            if depth == 0:
                return
            for index in range(arity):
                child = struct.unpack("Q", read(term + 104 + 8 * index, 8))[0]
                walk(child, depth - 1, prefix + f"  [{index}] ")

        walk(address, 6, "query ")

DumpTerm()
end

# At ClauseSetIndexedInsert+24 the optimized prologue has loaded the clause into
# r12. CLAUSE_PERM_IDENT puts ClauseCell.literals at +24; EqnCell stores its
# left and right terms at +8 and +16.
break *ClauseSetIndexedInsert+24
commands
  silent
  set $clause = (char *) $r12
  if *(long *) $clause == 2574
    set $literal = *(char **) ($clause + 24)
    set $rule_left = *(char **) ($literal + 8)
    set $rule_right = *(char **) ($literal + 16)
    printf "rule ident=2574 left=%p right=%p\n", $rule_left, $rule_right
  end
  continue
end

# SysV arguments are ocb, bank, lside, rside, subst in rdi..r8. Restrict this
# to clause 2574 considered from either indexed side.
break instance_is_rule
commands
  silent
  if $rule_right != 0 && (((char *) $rdx == $rule_left && (char *) $rcx == $rule_right) || ((char *) $rdx == $rule_right && (char *) $rcx == $rule_left))
    set $rule_hits = $rule_hits + 1
    set $awaiting_instance_compare = 1
    set $last_query = (char *) $rbx
    set $subst = (char *) $r8
    set $current = *(long *) ($subst + 8)
    set $stack = *(char ***) ($subst + 16)
    printf "instance hit=%ld direction=%s ordering=%d strong_rhs=%d lside=%p rside=%p subst=%p current=%ld\n", $rule_hits, ((char *) $rdx == $rule_left ? "left" : "right"), *(int *) $rdi, *(unsigned char *) ($rdi + 84), $rdx, $rcx, $subst, $current
    set $i = 0
    while $i < $current
      set $var = $stack[$i]
      set $binding = *(char **) ($var + 16)
      printf "  [%ld] var=%p f=%ld binding=%p", $i, $var, *(long *) $var, $binding
      if $binding != 0
        printf " binding_f=%ld binding_arity=%d", *(long *) $binding, *(int *) ($binding + 12)
      end
      printf "\n"
      set $i = $i + 1
    end
  end
  continue
end

# At +92, al is the just-returned SubstIsRenaming result and r14/r15 retain
# lside/subst from the function prologue.
break *instance_is_rule+92
commands
  silent
  if $rule_right != 0 && (((char *) $r14 == $rule_left && (char *) $rbx == $rule_right) || ((char *) $r14 == $rule_right && (char *) $rbx == $rule_left))
    printf "  SubstIsRenaming=%d\n", $al
    if $al != 0
      set $awaiting_instance_compare = 0
    end
  end
  continue
end

# KBO6Greater+85 has returned from kbolincmp but has not yet converted C's
# CompareResult in eax to a boolean. C uses 3 for to_greater.
break *KBO6Greater+85
commands
  silent
  if $awaiting_instance_compare != 0 && (((char *) $rbp == $rule_left && (char *) $r12 == $rule_right) || ((char *) $rbp == $rule_right && (char *) $r12 == $rule_left))
    printf "  KBO6Compare=%d\n", $eax
    set $awaiting_instance_compare = 0
    if $eax == 3
      dump-term $last_query
    end
  end
  continue
end

# All accepted indexed candidates converge at +248. ClausePos stores clause,
# literal, and side at +0, +8, and +16, respectively.
break *indexed_find_demodulator+248
commands
  silent
  set $accepted_pos = (char *) $r15
  if $accepted_pos != 0
    set $accepted_clause = *(char **) $accepted_pos
    if *(long *) $accepted_clause == 2574
      set $accepted_eqn = *(char **) ($accepted_pos + 8)
      printf "accepted ident=2574 side=%d eqn_props=%u oriented=%d query=%p\n", *(int *) ($accepted_pos + 16), *(unsigned int *) $accepted_eqn, ((*(unsigned int *) $accepted_eqn & 16) != 0), $rbx
      dump-term $rbx
      quit
    end
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
