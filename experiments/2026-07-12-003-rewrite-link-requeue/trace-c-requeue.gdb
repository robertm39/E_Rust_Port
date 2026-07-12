set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

set $clause680_left = 0
set $clause680_right = 0
set $tracking2574 = 0

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
            props = struct.unpack("I", read(term + 8, 4))[0]
            arity = struct.unpack("i", read(term + 12, 4))[0]
            replacement = struct.unpack("Q", read(term + 64, 8))[0]
            demod = struct.unpack("Q", read(term + 72, 8))[0]
            demod_id = struct.unpack("q", read(demod, 8))[0] if demod else 0
            gdb.write(
                f"{prefix}{term:#x} f={f_code} arity={arity} props={props:#x} "
                f"replacement={replacement:#x} demod={demod_id}\n"
            )
            if depth == 0:
                return
            for index in range(arity):
                child = struct.unpack("Q", read(term + 104 + 8 * index, 8))[0]
                walk(child, depth - 1, prefix + f"  [{index}] ")

        walk(address, 8, "")

DumpTerm()
end

# The optimized prologue retains the inserted clause in r12 at +24.
break *ClauseSetIndexedInsert+24
commands
  silent
  set $inserted = (char *) $r12
  if *(long *) $inserted == 680
    set $literal = *(char **) ($inserted + 24)
    set $clause680_left = *(char **) ($literal + 8)
    set $clause680_right = *(char **) ($literal + 16)
    printf "insert clause=680 left=%p right=%p\n", $clause680_left, $clause680_right
    dump-term $clause680_left
    dump-term $clause680_right
  end
  continue
end

# SysV arguments are ocb, index, stack, new_demod, nf_date.
break FindRewritableClausesIndexed
commands
  silent
  if *(long *) $rcx == 2574
    set $tracking2574 = 1
    printf "find demod=2574 date=%ld\n", $r8
  end
  continue
end

# Source, replacement, demodulator, SOS, and result are in rdi..r8.
break TermAddRWLink
commands
  silent
  if $rdx != 0 && *(long *) $rdx == 2574
    printf "link demod=2574 source=%p replacement=%p result=%d\n", $rdi, $rsi, $r8d
    dump-term $rdi
    printf "replacement tree\n"
    dump-term $rsi
  end
  continue
end

# The Find call has returned; r13 is the candidate stack and its data starts at
# +16. This point precedes the first pop in RemoveRewritableClausesIndexed.
break *RemoveRewritableClausesIndexed+138
commands
  silent
  if $tracking2574 != 0
    set $count = *(long *) ($r13 + 8)
    set $data = *(char ***) ($r13 + 16)
    printf "candidate count=%ld", $count
    set $i = 0
    while $i < $count
      printf " %ld", *(long *) $data[$i]
      set $i = $i + 1
    end
    printf "\n"
    if $clause680_left != 0
      printf "clause680 after find\n"
      dump-term $clause680_left
      dump-term $clause680_right
    end
    set $tracking2574 = 0
    quit
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
