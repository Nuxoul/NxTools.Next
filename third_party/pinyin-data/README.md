# pinyin-data provenance

MIT data, copyright (c) 2016 mozillazg, upstream https://github.com/mozillazg/pinyin-data.
Reviewed input copies from `D:\Code\WinShun-main\tools\data` on 2026-10-13; no upstream revision was recorded in that snapshot. Full independent license is LICENSE.

- pinyin.txt SHA256 `621F8CA9EFF8519F47E2B17B564FD318161E13BCA07EEA8C8E04993CD5D3B52E`
- kTGHZ2013.txt SHA256 `83BE5ECDFE0B8FCC1202AE45882A2F983C013DB87BA68369D94A4DAD775382BB`

`python scripts/generate-pinyin-data.py` regenerates Finder data; `--check` verifies exact outputs. No downloads or reference execution. Selection covers U+4E00..U+9FFF; standard characters retain up to three standard readings in common-first order, nonstandard ones retain one common reading. Tones removed, ü written v. Other characters retain the literal/legacy fallback, not guessed pronunciation. The independently implemented Rust matcher does not include WinShun source.
