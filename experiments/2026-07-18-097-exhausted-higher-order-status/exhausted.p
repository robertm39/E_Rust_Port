% Higher-order rewrite matching exhausts both axioms.
thf(a_type, type, a: $i).
thf(b_type, type, b: $i).
thf(c_type, type, c: $i).
thf(h_type, type, h: $i > $i).
thf(wrap_type, type, wrap: $i > $i).

thf(rewrite_rule, axiom,
    ! [F: $i > $i] : ((wrap @ (F @ a)) = (F @ b))).

thf(rewrite_target, axiom,
    ((wrap @ (h @ a)) = c)).
