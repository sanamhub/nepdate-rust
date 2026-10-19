#!/usr/bin/env python3
"""Render the W6-01 software versus femtovg comparison sheets.

The popup and font-test snapshots are produced by the app's snapshot feature:
`--popup --snapshot ...` or `--font-test --snapshot ...`. Software images are
rendered now with `SLINT_BACKEND=winit-software`; the right side is always a
committed femtovg image. The two 2x font-test reference images were rendered
with femtovg before W6-02 and committed under `femtovg/`.

Usage, from the repository root:

    python docs/plan/spike-results/w6/render_sheets.py \
        --binary D:/devcache/cargo-target-w6/debug/mitibar.exe

Requires Pillow. The script writes only under docs/plan/spike-results/w6/ and
uses a temporary directory for the raw renders.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
SHEETS = OUT / "sheets"


@dataclass(frozen=True)
class Case:
    name: str
    args: tuple[str, ...]
    scale: str
    right: str
    right_label: str = "femtovg (committed)"


POPUP = ("--popup",)
CONVERTER = ("--popup", "--converter")
SETTINGS = ("--popup", "--settings")
FONT = ("--font-test",)
MONTH = ("--today", "2083-07-01")
SIX_ROW = ("--today", "2081-01-15")
DETAIL = ("--today", "2081-01-01", "--detail", "2081-01-01")
DETAIL_MAX = ("--today", "2081-06-01", "--detail", "2081-06-01")
W3_DATE = ("--today", "2081-01-01")


def w2(name: str, today: tuple[str, ...], theme: str, lang: str, scale: str) -> Case:
    return Case(
        name,
        POPUP + today + ("--theme", theme, "--lang", lang),
        scale,
        f"w2/{name}.png",
    )


COMMITTED_W2 = {
    "month-grid-6row-light-en-1x",
    "month-grid-6row-light-ne-2x",
    "day-detail-light-en-1x",
    "day-detail-dark-ne-2x",
    "day-detail-max-light-en-1x",
    "day-detail-max-light-ne-2x",
}


def w2_extra(
    name: str, today: tuple[str, ...], theme: str, lang: str, scale: str
) -> Case:
    if name in COMMITTED_W2:
        return w2(name, today, theme, lang, scale)
    return Case(
        name,
        POPUP + today + ("--theme", theme, "--lang", lang),
        scale,
        f"w6/femtovg/{name}.png",
        "femtovg (rendered for W6-01)",
    )


def w3(
    name: str,
    mode: tuple[str, ...],
    theme: str,
    lang: str,
    scale: str,
    today: tuple[str, ...] = W3_DATE,
) -> Case:
    return Case(
        name,
        mode + today + ("--theme", theme, "--lang", lang),
        scale,
        f"w3/{name}.png",
    )


CASES: tuple[Case, ...] = (
    # W2: month grid, all eight combinations.
    *(
        w2(f"month-grid-{theme}-{lang}-{scale}", MONTH, theme, lang, scale)
        for theme in ("light", "dark")
        for lang in ("en", "ne")
        for scale in ("1x", "2x")
    ),
    # W2: the six-row month and the two day-detail cards, at all eight
    # combinations. The six images that were not committed before W6-01 have a
    # femtovg reference committed under `femtovg/`.
    *(
        w2_extra(
            f"month-grid-6row-{theme}-{lang}-{scale}",
            SIX_ROW,
            theme,
            lang,
            scale,
        )
        for theme in ("light", "dark")
        for lang in ("en", "ne")
        for scale in ("1x", "2x")
    ),
    *(
        w2_extra(f"day-detail-{theme}-{lang}-{scale}", DETAIL, theme, lang, scale)
        for theme in ("light", "dark")
        for lang in ("en", "ne")
        for scale in ("1x", "2x")
    ),
    *(
        w2_extra(
            f"day-detail-max-{theme}-{lang}-{scale}",
            DETAIL_MAX,
            theme,
            lang,
            scale,
        )
        for theme in ("light", "dark")
        for lang in ("en", "ne")
        for scale in ("1x", "2x")
    ),
    # W3: converter and settings, all eight combinations each.
    *(
        w3(f"converter-{theme}-{lang}-{scale}", CONVERTER, theme, lang, scale)
        for theme in ("light", "dark")
        for lang in ("en", "ne")
        for scale in ("1x", "2x")
    ),
    *(
        w3(f"settings-{theme}-{lang}-{scale}", SETTINGS, theme, lang, scale)
        for theme in ("light", "dark")
        for lang in ("en", "ne")
        for scale in ("1x", "2x")
    ),
    # W3-00b: the three committed tithi images. `tithi-before` is a historical
    # layout; the current code differs by the known 4 px inset, but the glyphs
    # are still compared.
    w3("tithi-before-light-en-1x", POPUP, "light", "en", "1x", MONTH),
    w3("tithi-after-light-en-1x", POPUP, "light", "en", "1x", MONTH),
    w3("tithi-after-light-ne-1x", POPUP, "light", "ne", "1x", MONTH),
    # W6: the font test at each scale and theme. It has no language flag: the
    # window itself holds both the Devanagari and the Latin strings.
    *(
        Case(
            f"font-test-{theme}-{scale}",
            FONT + ("--theme", theme),
            scale,
            f"windows-{theme}.png"
            if scale == "1x"
            else f"w6/femtovg/font-test-{theme}-2x.png",
        )
        for theme in ("light", "dark")
        for scale in ("1x", "2x")
    ),
)


def font(size: int) -> ImageFont.ImageFont:
    path = Path("C:/Windows/Fonts/segoeui.ttf")
    if path.exists():
        return ImageFont.truetype(str(path), size)
    return ImageFont.load_default()


def run(case: Case, binary: Path, tmp: Path, backend: str = "software") -> Path:
    out = tmp / f"{case.name}-{backend}.png"
    env = os.environ.copy()
    env["SLINT_BACKEND"] = f"winit-{backend}"
    env["SLINT_SCALE_FACTOR"] = case.scale[0]
    command = [str(binary), *case.args, "--snapshot", str(out)]
    result = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{case.name} exited {result.returncode}:\n"
            f"{result.stdout}\n{result.stderr}"
        )
    if not out.exists():
        raise RuntimeError(f"{case.name} wrote no {out}")
    return out


def sheet(case: Case, left: Path, right: Path, right_label: str, out: Path) -> None:
    a = Image.open(left).convert("RGBA")
    b = Image.open(right).convert("RGBA")
    if a.size != b.size:
        raise RuntimeError(f"{case.name}: software {a.size}, femtovg {b.size}")

    margin, gap, header = 16, 24, 58
    width = margin * 2 + a.width + gap + b.width
    height = header + max(a.height, b.height) + margin
    page = Image.new("RGB", (width, height), "#ffffff")
    draw = ImageDraw.Draw(page)
    title = font(20)
    label = font(16)
    draw.text((margin, 7), case.name, fill="#111111", font=title)
    draw.text((margin, 34), "software (winit-software)", fill="#333333", font=label)
    draw.text(
        (margin + a.width + gap, 34), right_label, fill="#333333", font=label
    )
    page.paste(a.convert("RGB"), (margin, header))
    page.paste(b.convert("RGB"), (margin + a.width + gap, header))
    draw.rectangle(
        (margin, header, margin + a.width - 1, header + a.height - 1),
        outline="#777777",
    )
    draw.rectangle(
        (
            margin + a.width + gap,
            header,
            margin + a.width + gap + b.width - 1,
            header + b.height - 1,
        ),
        outline="#777777",
    )
    page.save(out, "PNG", optimize=True)


def diff(a: Path, b: Path) -> dict[str, int | float]:
    left = Image.open(a).convert("RGBA")
    right = Image.open(b).convert("RGBA")
    if left.size != right.size:
        return {"width": left.width, "height": left.height, "size_mismatch": 1}
    total = left.width * left.height
    different = 0
    clear = 0
    maximum = 0
    total_delta = 0
    for pa, pb in zip(left.getdata(), right.getdata(), strict=True):
        delta = max(abs(pa[i] - pb[i]) for i in range(4))
        if delta:
            different += 1
        if delta > 16:
            clear += 1
        maximum = max(maximum, delta)
        total_delta += delta
    return {
        "width": left.width,
        "height": left.height,
        "different_pixels": different,
        "different_percent": round(100.0 * different / total, 3),
        "clearly_different_pixels": clear,
        "max_channel_delta": maximum,
        "mean_abs_delta": round(total_delta / total, 4),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--only", help="render only the case with this name")
    parser.add_argument(
        "--make-femtovg-references",
        action="store_true",
        help="render the missing femtovg references; run before W6-02",
    )
    args = parser.parse_args()

    binary = args.binary.resolve()
    if not binary.exists():
        parser.error(f"no binary at {binary}")

    if args.make_femtovg_references:
        references = [
            c for c in CASES if c.right.startswith("w6/femtovg/")
        ]
        with tempfile.TemporaryDirectory(prefix="mitibar-w6-") as tmp:
            temp = Path(tmp)
            for index, case in enumerate(references, 1):
                print(
                    f"[{index}/{len(references)}] reference {case.name}",
                    flush=True,
                )
                rendered = run(case, binary, temp, "femtovg")
                target = OUT.parent / case.right
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(rendered.read_bytes())
        print(f"wrote {len(references)} femtovg references")
        return 0

    SHEETS.mkdir(parents=True, exist_ok=True)
    cases = [c for c in CASES if not args.only or c.name == args.only]
    if not cases:
        parser.error(f"no case named {args.only}")

    metrics: dict[str, dict[str, int | float]] = {}
    with tempfile.TemporaryDirectory(prefix="mitibar-w6-") as tmp:
        temp = Path(tmp)
        for index, case in enumerate(cases, 1):
            print(f"[{index}/{len(cases)}] {case.name}", flush=True)
            left = run(case, binary, temp)
            right = OUT.parent / case.right
            sheet(case, left, right, case.right_label, SHEETS / f"{case.name}.png")
            metrics[case.name] = diff(left, right)

    (OUT / "diff.json").write_text(
        json.dumps(metrics, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {len(cases)} sheets and diff.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
