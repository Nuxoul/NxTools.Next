from pathlib import Path
import os,json
root=Path(r'D:\Code\NxTools.Next-main')
scratch=Path(os.environ['PI_SCRATCH_DIR'])/'command-naming'
baseline=root/'.trellis/tasks/10-26-command-naming/evidence/baseline/rust/apps/nxtools-desktop/src/downloads/polish_tests.rs'
old=baseline.read_text(encoding='utf-8')
for label in ['\u9009\u62e9\u9ed8\u8ba4\u4fdd\u5b58\u76ee\u5f55','\u9009\u62e9 FFmpeg \u53ef\u6267\u884c\u6587\u4ef6']:
 assert old.count(label)==1
 old=old.replace(label,label+'\u2026')
preview=(scratch/'polish.format-preview.rs').read_text(encoding='utf-8')
assert preview.startswith(old),'Old Download test prefix changed outside exact two label ellipses'
print('PASS original Download test prefix plus exact two punctuation updates')
