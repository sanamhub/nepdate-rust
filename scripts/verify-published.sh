#!/usr/bin/env bash
# Builds a new cargo project against a version of nepdate and nepdate-patro published on
# crates.io, the way a user adds them, and checks one conversion and one patro lookup.
#
#   scripts/verify-published.sh 0.1.0-beta.1 [--wait 20]
#   scripts/verify-published.sh latest [--wait 20]
#   scripts/verify-published.sh latest --resolve
#   scripts/verify-published.sh latest --metadata
#
# latest is the version a plain `cargo add` picks: crates.io's max_stable_version, or its
# max_version while only pre-releases exist. The resolved version is printed before anything is
# installed. --resolve prints it alone and exits, so a workflow can pass it to the later jobs.
# --metadata reads the registry's metadata API and confirms that each crate file is listed and
# answers a HEAD request; it downloads no crate, so it adds nothing to crates.io's download count.
#
# The program is built, then run directly (target/release/<name>, .exe on Windows) rather than
# through `cargo run`, so the check sees the binary a user would run.
set -euo pipefail

registry="https://crates.io/api/v1/crates"
user_agent="nepdate-verify-published"

mode=install
version=""
wait_minutes=20
while [ $# -gt 0 ]; do
  case "$1" in
    --resolve) mode=resolve; shift ;;
    --metadata) mode=metadata; shift ;;
    --wait)
      wait_minutes="${2:?--wait needs a number}"
      shift 2
      ;;
    -*)
      echo "verify-published: unknown argument '$1'"
      exit 2
      ;;
    *)
      version="$1"
      shift
      ;;
  esac
done
version="${version:-latest}"

registry_json() {
  curl -fsS --retry 3 -H "User-Agent: ${user_agent}" "${registry}/$1"
}

# The version `cargo add nepdate` resolves: the newest stable release, or the newest version of
# any kind while the crate has no stable release yet.
latest_version() {
  local json stable
  json="$(registry_json nepdate)"
  stable="$(printf '%s' "$json" | jq -r '.crate.max_stable_version // empty')"
  if [ -z "$stable" ]; then
    stable="$(printf '%s' "$json" | jq -r '.crate.max_version')"
  fi
  printf '%s' "$stable"
}

if [ "$version" = latest ]; then
  version="$(latest_version)"
  if [ "$mode" != resolve ]; then
    echo "verify-published: crates.io latest is ${version}"
  fi
fi
if [ "$mode" = resolve ]; then
  printf '%s\n' "$version"
  exit 0
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "verify-published: '$version' is not a version like 0.1.0 or 0.1.0-beta.1"
  exit 2
fi

# The daily check: the metadata API still lists the version and the crate file is still served.
# A yanked version or a file that answers 404 fails here, a day before a user would find it.
if [ "$mode" = metadata ]; then
  for crate in nepdate nepdate-patro; do
    yanked="$(registry_json "$crate" | jq -r --arg v "$version" '.versions[] | select(.num == $v) | .yanked')"
    if [ -z "$yanked" ]; then
      echo "verify-published: crates.io does not list ${crate} ${version}"
      exit 1
    fi
    if [ "$yanked" != false ]; then
      echo "verify-published: crates.io lists ${crate} ${version} as yanked"
      exit 1
    fi
    file="https://static.crates.io/crates/${crate}/${crate}-${version}.crate"
    code="$(curl -sS -o /dev/null -w '%{http_code}' -I --retry 3 "$file")"
    if [ "$code" != 200 ]; then
      echo "verify-published: ${file} answered ${code}"
      exit 1
    fi
    echo "verify-published: ${crate} ${version} is listed and its crate file answers 200"
  done
  exit 0
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
