#!/usr/bin/env bash
# Builds a new cargo project against a version of nepdate and nepdate-patro published on
# crates.io, the way a user adds them, and checks one conversion and one patro lookup.
#
#   scripts/verify-published.sh 0.1.0-beta.1 [minutes to wait for the index, default 20]
#
# The program is built, then run directly (target/release/<name>, .exe on Windows) rather than
# through `cargo run`, so the check sees the binary a user would run.
set -euo pipefail

version="${1:-}"
wait_minutes="${2:-20}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "verify-published: '$version' is not a version like 0.1.0 or 0.1.0-beta.1"
  exit 2
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"
cargo new --quiet --bin verify-published
cd verify-published
# Its own target directory, whatever the caller sets; cargo.exe on Windows needs a Windows path.
target="$PWD/target"
if command -v cygpath > /dev/null; then target="$(cygpath -w "$target")"; fi
export CARGO_TARGET_DIR="$target"

# The sparse index usually serves a new version within a minute; retry once a minute.
for ((i = 1; ; i++)); do
  if cargo add --quiet "nepdate@=${version}" "nepdate-patro@=${version}"; then
    break
  fi
  if ((i >= wait_minutes)); then
    echo "verify-published: crates.io still lacks ${version} after ${wait_minutes} minutes"
    exit 1
  fi
  echo "verify-published: ${version} not in the index yet, retrying in 60 s"
  sleep 60
done

cat > src/main.rs <<'EOF'
// The quick start of the nepdate and nepdate-patro READMEs, printed so the caller can compare.
use nepdate::{Lang, NepaliDate};

fn main() {
    let date = NepaliDate::new(2081, 4, 15).expect("valid");
    let ad = date.to_gregorian();
    println!("date: 2081/04/15 -> AD {:04}-{:02}-{:02}", ad.year(), ad.month(), ad.day());
    let parsed: NepaliDate = "2081/04/15".parse().expect("valid");
    println!("parse: {}", parsed == date);
    let info = nepdate_patro::info(NepaliDate::new(2081, 1, 1).expect("valid"));
    println!("tithi: {}", info.tithi().map_or("none", |t| t.name(Lang::En)));
    println!("holiday: {}", info.is_public_holiday());
}
EOF

cargo build --quiet --release
exe="target/release/verify-published"
[[ -f "${exe}.exe" ]] && exe="${exe}.exe"
got="$("./${exe}" | tr -d '\r')"
want="date: 2081/04/15 -> AD 2024-07-30
parse: true
tithi: Panchami
holiday: true"
echo "$got"
if [[ "$got" != "$want" ]]; then
  echo "verify-published: output differs; expected:"
  echo "$want"
  exit 1
fi
resolved="$(cargo tree --quiet --depth 1 --prefix none | sort -u | grep -E '^nepdate(-patro)? v' || true)"
echo "$resolved"
for crate in nepdate nepdate-patro; do
  if ! grep -qx "${crate} v${version}" <<<"$resolved"; then
    echo "verify-published: ${crate} did not resolve to ${version} from crates.io"
    exit 1
  fi
done
echo "verify-published: ok, nepdate and nepdate-patro ${version} from crates.io"
