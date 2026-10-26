"""parity.py frames: the same inputs on the original and the patched game (margin 0, options 0)."""
import subprocess, sys
frames = int(sys.argv[1])
press = ['50-53:start', '110-113:start', f'300-{frames}:right+b'] + [f'{f}-{f + 12}:a' for f in range(310, frames, 45)]
def run(name, extra):
    cmd = ['/home/user/zaxnes/rust/target/release/zax-harness', 'render', '--rom',
           '/tmp/claude-0/roms/Adventure Island (USA).nes', '--run', str(frames), '--press', ','.join(press),
           '--out', '/tmp/claude-0/ai/out/x', '--dump-mem', f'/tmp/claude-0/ai/out/{name}.mem'] + extra
    subprocess.run(cmd, capture_output=True)
    return open(f'/tmp/claude-0/ai/out/{name}.mem', 'rb').read()[:0x800]
a = run('par_o', ['--no-builtin-patches'])
b = run('par_p', ['--advanced', '--hack-options', '0'] + sys.argv[2:])
diff = [i for i in range(0x800) if a[i] != b[i]]
print(frames, 'differences', len(diff), [hex(i) for i in diff[:20]], 'lives', a[0x3F], b[0x3F], 'sec', a[0x30], b[0x30])
