import subprocess, sys
from PIL import Image
area = int(sys.argv[1]); frames = [int(x) for x in sys.argv[2:]] or [1000, 2600]
out = Image.new('RGB', (448 * len(frames), 240 * 4))
for rnd in range(4):
    for i, n in enumerate(frames):
        r = subprocess.run(['python3', '/tmp/claude-0/ai/tools/tour.py', str(area), str(rnd), str(n), f'ar{area}', '96', '62'], capture_output=True, text=True)
        print(r.stdout.strip().splitlines()[-1])
        out.paste(Image.open(f'/tmp/claude-0/ai/out/ar{area}.png').convert('RGB'), (448 * i, 240 * rnd))
out.save(f'/tmp/claude-0/ai/out/area{area}.png')
