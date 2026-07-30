#!/usr/bin/env bash
set -Eeuo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: smoke.sh UMLAUT UMLAUT_PCL_LEMMA" >&2
    exit 2
fi

umlaut=$1
selector=$2
smoke_root=$(mktemp -d /opt/e-rust-port/lemma-watchlist-smoke.XXXXXX)
trap 'rm -rf -- "$smoke_root"' EXIT

cat >"$smoke_root/source.pcl" <<'EOF'
1 : : [++p(a)] : initial
2 : : [++q(a)] : initial
3 : : [++r(a)] : pm(1,2)
4 : : [++s(a)] : pm(1,3)
5 : : [++t(a)] : er(4)
EOF

"$selector" \
    --flat-lemmas \
    --max-lemmas=2 \
    --min-lemma-quality=0 \
    --tstp-out \
    --output-level=1 \
    "$smoke_root/source.pcl" >"$smoke_root/selected.txt"

grep -q '^cnf(' "$smoke_root/selected.txt"

cat >"$smoke_root/problem.p" <<'EOF'
cnf(a,axiom,p(a)).
cnf(n,negated_conjecture,~p(a)).
cnf(w,watchlist,p(a)).
cnf(s,watchlist,$false).
EOF

"$umlaut" \
    --expert-heuristic=UseWatchlist \
    --term-ordering=KBO6 \
    --pcl-out \
    --proof-object=1 \
    "--static-watchlist=Use inline watchlist type" \
    --soft-cpu-limit=2 \
    --cpu-limit=4 \
    "$smoke_root/problem.p" >"$smoke_root/proof.pcl"

grep -Eq '% SZS status (Theorem|Unsatisfiable|ContradictoryAxioms)' \
    "$smoke_root/proof.pcl"
grep -Eq '^[[:space:]]*[0-9]+[[:space:]]*:' "$smoke_root/proof.pcl"

echo "OK: selector and inline static watchlist production paths"
