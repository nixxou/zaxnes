"""Test programs for the CPU netlist differential: each a 32 KiB-minus-vectors image, code at
$8000 (offset 0), handlers at $9000 (offset 0x1000)."""
import sys, os

def image(code, handler=b'\x40'):
    prg = bytearray(0x7FFA)
    code = bytes([0xA2, 0xFF, 0x9A]) + code  # LDX #$FF, TXS: the stack is not the same at power-on
    prg[:len(code)] = code
    prg[0x1000:0x1000 + len(handler)] = handler
    return bytes(prg)

out = sys.argv[1]
os.makedirs(out, exist_ok=True)

# 1. A copy loop: absolute,X reads with page crossings, stores, INX/BNE, JMP.
loop = bytes([
    0xA2, 0xF0,             # LDX #$F0
    0xBD, 0x20, 0x02,       # loop: LDA $0220,X  (crosses a page from X=$E0)
    0x9D, 0x00, 0x03,       # STA $0300,X
    0xE8,                   # INX
    0xD0, 0xF7,             # BNE loop
    0x4C, 0x03, 0x80,       # JMP $8003
])
open(os.path.join(out, 'loop.bin'), 'wb').write(image(loop))

# 2. Every addressing mode of ADC, read-modify-write, stack, JSR/RTS.
modes = bytes([
    0xA9, 0x10, 0x85, 0x10, 0xA9, 0x02, 0x85, 0x11,   # ($10) = $0210
    0xA0, 0xFF, 0xA2, 0x05,                           # Y=$FF X=5
    0x69, 0x01, 0x65, 0x10, 0x75, 0x10, 0x6D, 0x00, 0x02, 0x7D, 0x00, 0x02,
    0x79, 0x00, 0x02, 0x61, 0x0B, 0x71, 0x10,         # ADC #, zp, zp,X, abs, abs,X, abs,Y (cross), (zp,X), (zp),Y (cross)
    0xE6, 0x20, 0xF6, 0x20, 0xEE, 0x00, 0x03, 0xFE, 0x00, 0x03,  # INC zp, zp,X, abs, abs,X
    0x48, 0x68, 0x08, 0x28,                           # PHA PLA PHP PLP
    0x20, 0x43, 0x80,                                 # JSR $8043
    0x4C, 0x03, 0x80,                                 # JMP $8003
])
modes = modes + bytes(0x40 - len(modes)) + bytes([0xEA, 0x60])  # $8040: NOP RTS
open(os.path.join(out, 'modes.bin'), 'wb').write(image(modes))

# 3. BRK and NMI: NMI on ($2000 = $80), BRK, a spin; handler RTI.
nmi = bytes([
    0xA9, 0x80, 0x8D, 0x00, 0x20,   # $2000 = $80
    0x00, 0xEA,                     # BRK (+ signature byte)
    0xE8, 0x4C, 0x0A, 0x80,         # spin: INX, JMP spin
])
open(os.path.join(out, 'nmi.bin'), 'wb').write(image(nmi))

# 4. OAM DMA from RAM page 2, then a spin.
dma = bytes([
    0xA9, 0x02, 0x8D, 0x14, 0x40,   # $4014 = 2
    0xE8, 0x4C, 0x08, 0x80,         # spin
])
open(os.path.join(out, 'dma.bin'), 'wb').write(image(dma))
print('ok')
