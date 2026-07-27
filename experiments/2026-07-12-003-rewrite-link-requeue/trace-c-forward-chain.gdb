set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null

set $tracking680 = 0

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

        walk(address, 10, "")

DumpTerm()
end

# SysV arguments are state, control, clause, and contraction options.
break ForwardContractClause
commands
  silent
  if $tracking680 != 0 && *(long *) $rdx != 680
    printf "next selected old clause=%ld\n", *(long *) $rdx
    quit
  end
  if *(long *) $rdx == 680
    set $tracking680 = 1
    set $literal = *(char **) ($rdx + 24)
    set $left = *(char **) ($literal + 8)
    set $right = *(char **) ($literal + 16)
    printf "selected clause=680 left=%p right=%p\n", $left, $right
    printf "left before contraction\n"
    dump-term $left
    printf "right before contraction\n"
    dump-term $right
  end
  continue
end

# Source, replacement, demodulator, SOS, and result are in rdi..r8.
break TermAddRWLink
commands
  silent
  if $tracking680 != 0
    set $demod_id = 0
    if $rdx != 0
      set $demod_id = *(long *) $rdx
    end
    printf "add link source=%p replacement=%p demod=%ld result=%d\n", $rdi, $rsi, $demod_id, $r8d
    printf "source tree\n"
    dump-term $rdi
    printf "replacement tree\n"
    dump-term $rsi
  end
  continue
end

run --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
