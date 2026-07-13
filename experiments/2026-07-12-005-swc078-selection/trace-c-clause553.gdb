set pagination off
set confirm off
set debuginfod enabled off
set inferior-tty /dev/null
set $tracking = 0

python
import gdb
import struct

class DumpClauseLinks(gdb.Command):
    def __init__(self):
        super().__init__("dump-clause-links", gdb.COMMAND_DATA)

    def invoke(self, argument, from_tty):
        inferior = gdb.selected_inferior()

        def read(address, size):
            return bytes(inferior.read_memory(address, size))

        def u64(address):
            return struct.unpack("Q", read(address, 8))[0]

        def i64(address):
            return struct.unpack("q", read(address, 8))[0]

        def i32(address):
            return struct.unpack("i", read(address, 4))[0]

        def walk(term, depth, prefix):
            replacement = u64(term + 64)
            demod = u64(term + 72)
            demod_id = i64(demod) if demod else 0
            gdb.write(
                f"{prefix}{term:#x} f={i64(term)} arity={i32(term + 12)} "
                f"nf=[{i64(term + 48)},{i64(term + 56)}] "
                f"replacement={replacement:#x} demod={demod_id}\n"
            )
            if depth == 0:
                return
            for index in range(i32(term + 12)):
                walk(u64(term + 104 + 8 * index), depth - 1, prefix + f"  [{index}] ")

        clause = int(gdb.parse_and_eval(argument))
        literal = u64(clause + 24)
        index = 0
        while literal:
            gdb.write(f"literal {index} left\n")
            walk(u64(literal + 8), 3, "  ")
            gdb.write(f"literal {index} right\n")
            walk(u64(literal + 16), 3, "  ")
            literal = u64(literal + 32)
            index += 1

        derivation = u64(clause + 72)
        if derivation:
            current = i64(derivation + 8)
            values = u64(derivation + 16)
            gdb.write(f"derivation current={current}\n")
            for index in range(current):
                value = u64(values + 8 * index)
                if value > 0x10000:
                    try:
                        parent_id = i64(value)
                    except gdb.MemoryError:
                        parent_id = 0
                    gdb.write(f"  [{index}] pointer={value:#x} parent_id={parent_id}\n")
                else:
                    gdb.write(f"  [{index}] value={value}\n")

DumpClauseLinks()
end

# SysV arguments are state, control, clause, and contraction options.
break ForwardContractClause
commands
  silent
  set $clause = (char *) $rdx
  if *(long *) $clause == -9223372036854775255
    set $tracking = 1
    enable 2
    printf "before contraction ident=553\n"
    dump-clause-links $clause
  end
  continue
end

# TermAddRWLink(rewritten, replacement, demodulator, restricted, sos_rewritten).
break TermAddRWLink
commands
  silent
  if $tracking != 0 && $rdx != 0
    printf "rewrite demod=%ld source=%p replacement=%p\n", *(long *) $rdx, $rdi, $rsi
  end
  continue
end
disable 2

# document_processing receives the surviving post-contraction clause in rdi.
break *ProcessClause+2112
commands
  silent
  set $clause = (char *) $r14
  if *(long *) $clause == -9223372036854775255
    printf "after contraction ident=553\n"
    dump-clause-links $clause
    quit
  end
  continue
end

run --auto --output-level=1 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p
