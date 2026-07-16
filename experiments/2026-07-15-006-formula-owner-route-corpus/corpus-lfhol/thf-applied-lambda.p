% Status : Theorem
thf(f_type, type, f: $i > $i).
thf(h_type, type, h: ($i > $i) > $o).
thf(source, axiom,
    h @ (^[X: $i]: (f @ X))).
thf(goal, conjecture,
    h @ (^[X: $i]: (f @ X))).
