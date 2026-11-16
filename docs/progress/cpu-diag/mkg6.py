"""G6 programs: which bus gives a controller read its open bits."""
import sys, os
out = sys.argv[1]

def image(code):
    prg = bytearray(0x7FFA)
    code = bytes([0xA2, 0xFF, 0x9A]) + code
    prg[:len(code)] = code
    prg[0x1000] = 0x40
    return bytes(prg)

def save(name, data):
    open(os.path.join(out, name + '.bin'), 'wb').write(data)

SETUP = bytes([
    0xA9, 0x40, 0x8D, 0x17, 0x40,   # frame IRQ inhibited
    0xA9, 0x0F, 0x8D, 0x10, 0x40,   # DMC rate 15, no loop
    0xA9, 0x00, 0x8D, 0x12, 0x40,   # sample at $C000
    0xA9, 0xFF, 0x8D, 0x13, 0x40,   # longest sample
    0xA9, 0x10, 0x8D, 0x15, 0x40,   # DMC on: $4015 now reads $10 (BPL)
])
# Opcode fetched from $4015 ($10, BPL), its operand from $4016: the branch's target shows what
# the CPU took for the operand's open bits.
save('g6jmp', image(SETUP + bytes([0xA9, 0x00, 0x4C, 0x15, 0x40])))
# The same with the DMC off: $4015 reads $00 (BRK), the next fetches are $4016/$4017.
save('g6brk', image(SETUP[:20] + bytes([0xA9, 0x00, 0x8D, 0x15, 0x40, 0xA9, 0x00, 0x4C, 0x15, 0x40])))
print('ok')
