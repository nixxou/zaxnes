"""tour.py area round frames name [margin] [speedX]: start a round by pokes, scroll it by force."""
import subprocess, sys
area, rnd, frames, name = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
margin = sys.argv[5] if len(sys.argv) > 5 else '96'
speed = sys.argv[6] if len(sys.argv) > 6 else '70'
pokes = []
for f in range(116, 160):
    pokes += [f'{f}:37={area:02X}', f'{f}:38={rnd:02X}']
for f in range(250, frames):
    pokes += [f'{f}:72=00']
    if f % 2 == 0:
        pokes += [f'{f}:584={speed}']
    if f % 64 == 0:
        pokes += [f'{f}:3F=05', f'{f}:76=0C']
cmd = ['/home/user/zaxnes/rust/target/release/zax-harness', 'render', '--rom',
       '/tmp/claude-0/roms/Adventure Island (USA).nes', '--run', str(frames), '--press', '50-53:start,110-113:start',
       '--poke', ','.join(pokes), '--out', '/tmp/claude-0/ai/out/x', '--widescreen', margin,
       '--png-nes', f'/tmp/claude-0/ai/out/{name}.png', '--dump-mem', f'/tmp/claude-0/ai/out/{name}.mem'] + sys.argv[7:]
r = subprocess.run(cmd, capture_output=True, text=True); print(r.stderr[-3000:]) if 'AI_DEBUG' in __import__('os').environ else None
d = open(f'/tmp/claude-0/ai/out/{name}.mem', 'rb').read()
print(name, 'area', d[0x37], 'round', d[0x38], 'sec', hex(d[0x30]), 'col', hex(d[0x35]), '31/32', hex(d[0x31]), hex(d[0x32]), 'sY', d[7], 'death', d[0x72], 'bonus', d[0x49])
