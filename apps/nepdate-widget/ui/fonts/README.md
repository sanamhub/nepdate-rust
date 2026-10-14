# Bundled fonts

Both fonts are variable TTFs under the SIL Open Font License 1.1. Each `OFL.txt` is next to its
font. Downloaded from the official `notofonts` GitHub releases on 2026-10-15.

| File | Family | Version | Source |
|---|---|---|---|
| `NotoSans-Variable.ttf` | Noto Sans (Latin, Greek, Cyrillic), weight axis | v2.015 | https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSans-v2.015 |
| `NotoSansDevanagari-Variable.ttf` | Noto Sans Devanagari, weight axis | v2.007 | https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.007 |

The release zip names the files `NotoSans[wght].ttf` and `NotoSansDevanagari[wght].ttf`. The
brackets are renamed to hyphens here because `import` with bracketed filenames is unverified
(W0-02). The files are the `slim-variable-ttf` builds: one weight axis, no width axis, the
smallest variable build.
