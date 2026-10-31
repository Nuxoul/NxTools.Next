from pathlib import Path
import json
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-rem-archive-final-matrix'
rows=json.loads((task/'evidence/desktop-file-coverage-v2.json').read_text(encoding='utf-8'))
lines=['# Per-file #6 role index — every unfinished file is explicit','','Status is conservative. This is a navigation projection of desktop-file-coverage-v2.json, not semantic or compiled acceptance. Original bounded source/role/test mappings are in role-matrix.md. A TEST_ONLY filename classification is lexical; production reachability of conditional/development fixtures still needs actual caller review before using it as an exemption. No direct px calls does not clear indirect units/constants/inherited typography. All non-test file remainders below remain UNFINISHED_REVIEW unless an exact bounded role is separately evidenced in role-matrix.md.','','| File under Desktop src | Direct px calls | Lexical test calls | Per-file residual status |','| --- | ---: | ---: | --- |']
for row in rows:
 rel=row['path'].split('/src/',1)[1]
 test=row['classification_counts'].get('TEST_ONLY',0)
 status='TEST_ONLY_FILENAME; not production coverage' if row['status']=='TEST_ONLY' else 'UNFINISHED_REVIEW; no direct px is not clearance' if row['px_calls']==0 else 'UNFINISHED_REVIEW / see exact-site ledger and bounded role matrix'
 lines.append(f'| `{rel}` | {row["px_calls"]} | {test} | {status} |')
path=task/'file-role-index.md';assert not path.exists();path.write_text('\n'.join(lines)+'\n',encoding='utf-8')
print(f'Created file-role-index.md: {len(rows)} file rows')
