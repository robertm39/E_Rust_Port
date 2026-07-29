% Proof-only differential witness. It is excluded from performance decisions.
cnf(north_east, axiom, p(X) | q(X)).
cnf(north_west, axiom, ~p(a) | q(a)).
cnf(south_east, axiom, p(a) | ~q(a)).
cnf(south_west, axiom, ~p(a) | ~q(a)).
