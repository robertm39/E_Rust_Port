tff(person_type, type, person: $tType).
tff(p_type, type, p: person > $o).
tff(q_type, type, q: person > $o).
tcf(bad, axiom, ![X: person]:(p(X) & q(X))).
