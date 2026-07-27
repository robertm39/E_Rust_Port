set pagination off
break TypeBankPrintSelectedSortDefs
run

python
import gdb

inferior = gdb.selected_inferior()

def unsigned_word(address):
    return int.from_bytes(inferior.read_memory(address, 8), "little")

def signed_word(address):
    return int.from_bytes(inferior.read_memory(address, 8), "little", signed=True)

def signed_int(address):
    return int.from_bytes(inferior.read_memory(address, 4), "little", signed=True)

def visit(node):
    if node == 0:
        return
    visit(unsigned_word(node))
    key = unsigned_word(node + 16)
    f_code = signed_word(key)
    arity = signed_int(key + 8)
    type_uid = signed_word(key + 24)
    if arity == 0 and f_code > 6:
        print(f"sort address=0x{key:x} f_code={f_code} type_uid={type_uid}")
    visit(unsigned_word(node + 8))

visit(int(gdb.parse_and_eval("$rdx")))
end
