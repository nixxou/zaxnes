"""dmc0/dmc1: the DMC program with the $4015 write on one CPU cycle, or one later (a page-crossing
LDA abs,X costs one cycle more), so both get/put parities of the load are judged."""
import sys, os
out = sys.argv[1]

def image(code):
    prg = bytearray(0x7FFA)
    code = bytes([0xA2, 0xFF, 0x9A]) + code
    prg[:len(code)] = code
    prg[0x1000] = 0x40
    return bytes(prg)

for name, x in (('dmc0', 0x00), ('dmc1', 0x01)):
    code = bytes([
        0xA2, x, 0xBD, 0xFF, 0x02,      # LDX #x, LDA $02FF,X (4 or 5 cycles)
        0xA9, 0x4F, 0x8D, 0x10, 0x40,   # $4010 = $4F
        0xA9, 0x00, 0x8D, 0x12, 0x40,   # $4012 = 0
        0xA9, 0x00, 0x8D, 0x13, 0x40,   # $4013 = 0
        0xA9, 0x10, 0x8D, 0x15, 0x40,   # $4015 = $10
        0xAD, 0x00, 0x02,               # loop: LDA $0200
        0x8D, 0x01, 0x02,               # STA $0201
        0xEE, 0x02, 0x02,               # INC $0202
        0x48, 0x68,                     # PHA PLA
        0x4C, 0x1C, 0x80,               # JMP loop ($801C)
    ])
    open(os.path.join(out, name + '.bin'), 'wb').write(image(code))
print('ok')
