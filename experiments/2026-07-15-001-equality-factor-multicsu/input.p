% Multi-CSU equality-factor trace fixture.
thf(a_type, type, a: $i).
thf(b_type, type, b: $i).
thf(c_type, type, c: $i).
thf(d_type, type, d: $i).
thf(e_type, type, e: $i).
thf(q_type, type, q: $i > $i).

thf(equality_factor_seed, axiom,
    ! [F: $i > $i] :
      (((q @ (q @ (q @ (F @ a)))) = (d))
      | ((q @ (q @ (q @ a))) = (c))
      | ((F @ b) != (e)))).
