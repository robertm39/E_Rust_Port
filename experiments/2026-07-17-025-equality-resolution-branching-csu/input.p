% Eligible branching-CSU equality-resolution fixture.
thf(a_type, type, a: $i).
thf(b_type, type, b: $i).
thf(e_type, type, e: $i).

thf(equality_resolution_seed, axiom,
    ! [F: $i > $i] :
      (((F @ b) = e)
      | ((F @ a) != a))).
