% Direct LFHO complete-MGU paramodulation: F @ b unifies with h @ a @ b.
thf(a_type, type, a: $i).
thf(b_type, type, b: $i).
thf(c_type, type, c: $i).
thf(d_type, type, d: $i).
thf(h_type, type, h: $i > $i > $i).
thf(p_type, type, p: $o).

thf(source, axiom,
    ! [F: $i > $i] : (((F @ b) = d) | p)).

thf(target, axiom,
    ((h @ a @ b) = c)).
