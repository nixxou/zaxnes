"""Prototype: lay out a whole Adventure Island stage from RAM pointers + PRG; draw it."""
import sys
from PIL import Image
rom = open('/tmp/claude-0/roms/Adventure Island (USA).nes','rb').read()
prg = rom[16:16+0x8000]; chr_ = rom[16+0x8000:]
ram = open(sys.argv[1],'rb').read()[:0x800]
def rd(a): return prg[a-0x8000]
def w(a): return ram[a] | ram[a+1]<<8
p1A, p1C, p1E, p20, p22, p24, p26 = (w(a) for a in (0x1A,0x1C,0x1E,0x20,0x22,0x24,0x26))
sections = ram[0x2F]
def column(W):
    s = W >> 4
    screen = rd(p1A + 3*s) >> 2
    b = [rd(p1C + 3*screen + i) for i in range(3)]
    v = b[0]<<16 | b[1]<<8 | b[2]
    k = (W >> 2) & 3
    struct = (v >> (18 - 6*k)) & 0x3F
    top = rd(p1E + 2*struct); bot = rd(p1E + 2*struct + 1)
    tl = [rd(p20 + 6*top + i) for i in range(6)]
    bl = [rd(p22 + 5*bot + i) for i in range(5)]
    metas = [tl[0], tl[0], tl[0]] + tl[1:6] + bl + [bl[4]]
    c = W & 3
    tiles = []; pals = []
    for m in metas:
        for r in range(4):
            tiles.append(rd(p24 + 16*m + c + 4*r))
        a = rd(p26 + m)
        sh = 2 if (W >> 2) & 1 else 0
        pals += [(a >> sh) & 3]*2 + [(a >> (sh+4)) & 3]*2
    return tiles, pals
bank = ram[0x0A] & 3
pal = ram[0x140:0x160]
import colorsys
NES = [(84,84,84),(0,30,116),(8,16,144),(48,0,136),(68,0,100),(92,0,48),(84,4,0),(60,24,0),(32,42,0),(8,58,0),(0,64,0),(0,60,0),(0,50,60),(0,0,0),(0,0,0),(0,0,0),(152,150,152),(8,76,196),(48,50,236),(92,30,228),(136,20,176),(160,20,100),(152,34,32),(120,60,0),(84,90,0),(40,114,0),(8,124,0),(0,118,40),(0,102,120),(0,0,0),(0,0,0),(0,0,0),(236,238,236),(76,154,236),(120,124,236),(176,98,236),(228,84,236),(236,88,180),(236,106,100),(212,136,32),(160,170,0),(116,196,0),(76,208,32),(56,204,108),(56,180,204),(60,60,60),(0,0,0),(0,0,0),(236,238,236),(168,204,236),(188,188,236),(212,178,236),(236,174,236),(236,174,212),(236,180,176),(228,196,144),(204,210,120),(180,222,120),(168,226,144),(152,226,180),(160,214,228),(160,162,160),(0,0,0),(0,0,0)]
ncols = sections*16
img = Image.new('RGB', (ncols*8, 56*8))
pt = 0x1000 if ram[4] & 0x10 else 0
for W in range(ncols):
    tiles, pals = column(W)
    for j,(t,p) in enumerate(zip(tiles,pals)):
        base = bank*0x2000 + pt + t*16
        for y in range(8):
            lo = chr_[base+y]; hi = chr_[base+y+8]
            for x in range(8):
                ci = ((lo >> (7-x)) & 1) | (((hi >> (7-x)) & 1) << 1)
                col = pal[0] if ci == 0 else pal[p*4+ci]
                img.putpixel((W*8+x, j*8+y), NES[col & 0x3F])
img.save(sys.argv[2])
print('sections', sections, 'ptrs', [hex(x) for x in (p1A,p1C,p1E,p20,p22,p24,p26)], '31..35', [hex(ram[a]) for a in range(0x30,0x36)])
