% Status : Theorem
thf(f_type, type, f: $i > $i).
thf(g_type, type, g: $i > $i).
thf(source, axiom,
    (f = (^[X: $i]: (g @ X)))).
thf(goal, conjecture,
    (f = (^[X: $i]: (g @ X)))).
