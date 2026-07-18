tff(a_type, type, a: $i).
tff(c_type, type, c: $i).
tff(p_type, type, p: $i > $o).
tff(q_type, type, q: $i > $o).
tff(f_type, type, f: $o > $i).
fof(bool_arg, axiom, (f((p(a)&q(a))) = c)).
