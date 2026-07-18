thf(person_type, type, person: $tType).
thf(a_type, type, a: person).
thf(p_type, type, p: person > $o).
thf(q_type, type, q: person > $o).
thf(left, axiom, ((p @ a) | (q @ a))).
thf(right, axiom, (~(p @ a) | (q @ a))).
