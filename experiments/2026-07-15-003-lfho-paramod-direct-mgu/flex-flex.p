% Direct LFHO complete-MGU paramodulation: F @ a unifies with G @ b @ a.
thf(a_type, type, a: $i).
thf(b_type, type, b: $i).
thf(c_type, type, c: $i).
thf(d_type, type, d: $i).
thf(p_type, type, p: $o).

thf(source, axiom,
    ! [F: $i > $i] : (((F @ a) = d) | p)).

thf(target, axiom,
    ! [G: $i > $i > $i] : ((G @ b @ a) = c)).
