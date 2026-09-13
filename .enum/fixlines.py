import re, sys
path = sys.argv[1]; lines_spec = sys.argv[2]
word = sys.argv[3] if len(sys.argv) > 3 else 'progenitor'
MASK = ['trusted-init', 'init-measure', 'init-and-granular-spawn', 'pre-init', 'init-sipi-sipi',
        'linux,initrd', 'init-boot', 'init/hello']
targets = set()
for part in lines_spec.split(','):
    if '-' in part and part[0].isdigit() and part.split('-')[1].isdigit():
        a, b = part.split('-'); targets.update(range(int(a), int(b)+1))
    else:
        targets.add(int(part))
src = open(path).read().split('\n')
n = 0
for i in sorted(targets):
    idx = i - 1
    line = src[idx]
    holders = {}
    for k, m in enumerate(MASK):
        if m in line:
            key = f'\x00{k}\x00'; holders[key] = m; line = line.replace(m, key)
    new, cnt = re.subn(r'\binit\b', word, line)
    for key, m in holders.items():
        new = new.replace(key, m)
    if cnt == 0:
        print(f'WARN no match {path}:{i}: {src[idx]}')
    src[idx] = new; n += cnt
open(path, 'w').write('\n'.join(src))
print(f'{path}: {n} replaced on {len(targets)} lines')
