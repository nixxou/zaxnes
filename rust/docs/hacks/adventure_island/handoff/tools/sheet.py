"""sheet.py area round out.png frames...: tour a round, stack the captures."""
import subprocess, sys
from PIL import Image
area, rnd, out = sys.argv[1], sys.argv[2], sys.argv[3]
ims = []
for n in sys.argv[4:]:
    r = subprocess.run(['python3', '/tmp/claude-0/ai/tools/tour.py', area, rnd, n, f'sh{n}', '96', '62'], capture_output=True, text=True)
    print(r.stdout.strip().splitlines()[-1])
    ims.append(Image.open(f'/tmp/claude-0/ai/out/sh{n}.png').convert('RGB'))
w = Image.new('RGB', (ims[0].width, 240 * len(ims)))
for i, im in enumerate(ims):
    w.paste(im, (0, 240 * i))
w.save(out)
