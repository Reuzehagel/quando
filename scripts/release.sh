#!/usr/bin/env bash
# Tags the version in Cargo.toml once it is the bump its changes need since the last tag. Breaking is an API break
# (cargo-semver-checks) or a case in tests/cases.json whose expected result changed; new cases are not. In 0.x a
# breaking change bumps the minor (0.1 -> 0.2), in 1.x the major. Usage: bump Cargo.toml, commit, scripts/release.sh
set -euo pipefail
cd "$(dirname "$0")/.."

[ -z "$(git status --porcelain)" ] || { echo "commit or stash first: the tag goes on HEAD"; exit 1; }

old=$(git describe --tags --abbrev=0 --match 'v*')
new=v$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ "$(printf '%s\n%s\n' "$old" "$new" | sort -V | tail -1)" = "$new" ] && [ "$old" != "$new" ] \
  || { echo "Cargo.toml says $new, which is not after $old: bump it"; exit 1; }

IFS=. read -r o_major o_minor _ <<< "${old#v}"
IFS=. read -r n_major n_minor _ <<< "${new#v}"
if [ "$o_major" = 0 ] && [ "$n_major" = 0 ]; then [ "$n_minor" != "$o_minor" ] && breaking=1 || breaking=0
else [ "$n_major" != "$o_major" ] && breaking=1 || breaking=0; fi

changed=$(git show "$old:tests/cases.json" | node -e '
  const old = JSON.parse(require("fs").readFileSync(0, "utf8"));
  const now = JSON.parse(require("fs").readFileSync("tests/cases.json", "utf8"));
  const want = c => JSON.stringify([old.now, c.want]);
  const before = new Map(old.cases.map(c => [c.input, want(c)]));
  for (const c of now.cases)
    if (before.has(c.input) && before.get(c.input) !== JSON.stringify([now.now, c.want])) console.log("  " + c.input);
')
if [ -n "$changed" ] && [ $breaking = 0 ]; then
  echo "these cases read differently than in $old, which is breaking: $new needs a breaking bump"; echo "$changed"; exit 1
fi

command -v cargo-semver-checks >/dev/null || { echo "needs: cargo install cargo-semver-checks --locked"; exit 1; }
cargo semver-checks --baseline-rev "$old"
scripts/check.sh
git tag -a "$new" -m "quando ${new#v}"
echo "tagged $new; push with: git push origin main $new"
