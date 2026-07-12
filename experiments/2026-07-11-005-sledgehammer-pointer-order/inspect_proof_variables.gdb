set pagination off
break TypeBankPrintSelectedSortDefs
run
disable 1

python
import gdb

class DumpProofVariables(gdb.Command):
    def __init__(self):
        super().__init__("dump-proof-variables", gdb.COMMAND_DATA)
        self.call = 0

    def invoke(self, argument, from_tty):
        del argument, from_tty
        self.call += 1
        inferior = gdb.selected_inferior()

        def unsigned_word(address):
            return int.from_bytes(inferior.read_memory(address, 8), "little")

        def signed_word(address):
            return int.from_bytes(
                inferior.read_memory(address, 8), "little", signed=True
            )

        root = int(gdb.parse_and_eval("$rsi"))
        stack = [root]
        variables = []
        while stack:
            node = stack.pop()
            if node == 0:
                continue
            key = unsigned_word(node + 16)
            variables.append(
                (key, signed_word(key), signed_word(key + 24))
            )
            stack.append(unsigned_word(node))
            stack.append(unsigned_word(node + 8))

        if len(variables) >= 4 and all(f_code < 0 for _, f_code, _ in variables):
            rendered = " ".join(
                f"0x{address:x}:f{f_code}:e{entry_no}"
                for address, f_code, entry_no in variables
            )
            print(f"closure {self.call} variables={len(variables)} {rendered}")

DumpProofVariables()
end

break PTreeToPStack
commands 2
silent
dump-proof-variables
continue
end
continue
