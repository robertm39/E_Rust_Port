cnf(positive_gate, axiom, (p(X) | ~q(X))).
cnf(negative_gate, axiom, (~p(X) | q(X))).
cnf(p_offending, axiom, (p(a) | ~p(b))).
cnf(q_offending, axiom, (q(c) | q(d))).
