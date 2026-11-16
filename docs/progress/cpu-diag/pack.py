"""pack.py <dest.zip>: the proof's traces, netlist (n4-*) and Zaxnes with DMA detail (z5-*)."""
import sys, zipfile, os, hashlib
here = os.path.dirname(os.path.abspath(__file__))
programs = ['loop', 'modes', 'allops', 'branch', 'dma', 'oam0', 'oam1', 'nmi', 'irq', 'dmc', 'dmc0',
            'dmc1', 'dmcoam', 'dmc2007', 'illegal', 'dmc4016', 'intbranch', 'fc5', 'g6jmp', 'g6brk']
lines = []
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_LZMA) as z:
    for p in programs:
        for name in (f'{p}.bin', f'n4-{p}.txt', f'z5-{p}.txt'):
            path = os.path.join(here, name)
            data = open(path, 'rb').read()
            z.writestr(name, data)
            lines.append(f'{hashlib.sha256(data).hexdigest()}  {name}')
    z.writestr('SHA256SUMS', '\n'.join(lines) + '\n')
print(os.path.getsize(sys.argv[1]))
