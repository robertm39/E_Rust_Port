% Direct LFHO complete-MGU paramodulation through an eta-lambda/DB argument.
thf(c_type, type, c: $i).
thf(d_type, type, d: $i).
thf(h_type, type, h: $i > $i).
thf(wrap_type, type, wrap: ($i > $i) > $i).
thf(p_type, type, p: $o).

thf(source, axiom,
    ! [F: $i > $i] : (((wrap @ F) = d) | p)).

thf(target, axiom,
    ((wrap @ (^ [X: $i] : (h @ X))) = c)).

% The two ground literals make the fixture contradictory only after the
% paramodulant (c = d) | p has been generated and simplified.
thf(not_p, axiom, ~p).
thf(c_not_d, axiom, c != d).
