cnf(p_source,axiom,p(a)).
cnf(q_source,axiom,q(a)).
cnf(not_r_source,axiom,~r(a)).
cnf(goal,negated_conjecture,(~p(U)|r(U)|~q(U))).
