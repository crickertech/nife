import re, subprocess, sys
DET = {'the','The','a','A','an','An','its','Its','this','This','that','That','our','one','One',
       'each','Each','every','Every','no','No','any','Any','my','their','some','Some','another','Another'}
files = subprocess.run(['git','diff','--name-only'],capture_output=True,text=True).stdout.split()
pat = re.compile(r'(?<![\w`/\-\'"])progenitor\b')
changed=0
for p in files:
    if not p.endswith(('.rs','.md','.toml')): continue
    out=[]
    for line in open(p).read().split('\n'):
        def fix(m):
            global changed
            pre = line[:m.start()]
            # inside backticks / code path / already determined?
            if pre.endswith('`') or pre.endswith('/') or pre.endswith('-'): return m.group(0)
            words = pre.rstrip().split()
            if words and words[-1] in DET: return m.group(0)
            # start of a sentence or of a doc line
            stripped = pre.rstrip()
            if stripped.endswith(('//!','///','//','.','!','?',':','*','-',')','(', '**')) or stripped=='' :
                changed+=1
                return 'The progenitor' if stripped.endswith(('.','!','?','//!','///','//','**','*')) or stripped=='' else 'the progenitor'
            changed+=1
            return 'the progenitor'
        out.append(pat.sub(fix, line))
    open(p,'w').write('\n'.join(out))
print('inserted articles:', changed)
