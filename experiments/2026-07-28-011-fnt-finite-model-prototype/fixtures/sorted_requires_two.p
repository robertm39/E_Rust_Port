% Status   : Satisfiable
fof(two_p_elements,axiom,
    ? [X,Y] : ( p(X) & p(Y) & X != Y ) ).
fof(one_q_element,axiom,
    ? [Z] : q(Z) ).
