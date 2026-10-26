"""play.py level frames name [options] [margin] [extra...]: level select, hold right+B, jump often."""
import subprocess, sys
level, frames, name = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
options = sys.argv[4] if len(sys.argv) > 4 else '7'
margin = sys.argv[5] if len(sys.argv) > 5 else '96'
press = ['50-53:start'] + [f'{70 + 6 * i}:select' for i in range(level)] + [f'{80 + 6 * level}-{83 + 6 * level}:start']
press.append(f'300-{frames}:right+b')
press += [f'{f}-{f + 12}:a' for f in range(310, frames, 45)]
cmd = ['/home/user/zaxnes/rust/target/release/zax-harness', 'render', '--rom',
       '/tmp/claude-0/roms/Adventure Island (USA).nes', '--advanced', '--hack-options', options,
       '--run', str(frames), '--press', ','.join(press), '--out', '/tmp/claude-0/ai/out/x', '--widescreen', margin,
       '--png-nes', f'/tmp/claude-0/ai/out/{name}.png', '--dump-mem', f'/tmp/claude-0/ai/out/{name}.mem'] + sys.argv[6:]
r = subprocess.run(cmd, capture_output=True, text=True)
d = open(f'/tmp/claude-0/ai/out/{name}.mem', 'rb').read()
print(name, 'area', d[0x37], 'round', d[0x38], 'sec', hex(d[0x30]), 'lives', d[0x3F], 'death', d[0x72], 'stamina', d[0x76])
