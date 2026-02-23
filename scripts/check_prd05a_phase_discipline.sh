#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

branch="${GITHUB_HEAD_REF:-${GITHUB_REF_NAME:-$(git rev-parse --abbrev-ref HEAD)}}"
report_path="local/reports/prd05a/C5-phase-discipline-report.md"

mkdir -p local/reports/prd05a

status="PASS"
reason="phase/milestone branch and commit discipline checks passed"

if [[ "$branch" != "main" && "$branch" != "sign-tx" ]]; then
  if [[ ! "$branch" =~ ^feat/prd05a-e2e-(e[0-9]+|m[0-9]+)-[a-z0-9-]+$ ]]; then
    status="FAIL"
    reason="branch name does not match feat/prd05a-e2e-(e<phase>|m<milestone>)-<slug>: ${branch}"
  fi
fi

commit_sample="$(git log -n 30 --pretty=%s 2>/dev/null || true)"
if [[ "$status" == "PASS" ]]; then
  if ! echo "$commit_sample" | rg -q "E[0-9]-T[0-9]|M[0-9]-P[0-9]|gate-green"; then
    status="FAIL"
    reason="recent commit messages do not include E*-T*, M*-P*, or gate-green markers"
  fi
fi

cat >"$report_path" <<EOF
# C5 Phase Discipline Report

- Branch: \`${branch}\`
- Status: \`${status}\`
- Reason: ${reason}

## Commit Sample (latest 10)

\`\`\`
$(git log -n 10 --pretty=%h' '%s 2>/dev/null || true)
\`\`\`
EOF

if [[ "$status" == "PASS" ]]; then
  echo "phase discipline: PASS"
  exit 0
fi

echo "phase discipline: FAIL" >&2
echo "$reason" >&2
exit 1
