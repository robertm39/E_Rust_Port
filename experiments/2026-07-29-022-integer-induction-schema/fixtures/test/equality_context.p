tff(value_type,type,
    value: $tType ).

tff(z_type,type,
    z: value ).

tff(f_type,type,
    f: $int > value ).

tff(next_type,type,
    next: value > value ).

tff(wrap_type,type,
    wrap: value > value ).

tff(base,axiom,
    f(2) = z ).

tff(fixed,axiom,
    next(z) = z ).

tff(step,axiom,
    ! [N: $int] :
      ( $greatereq(N,2)
     => ( f($sum(N,1)) = next(f(N)) ) ) ).

tff(goal,conjecture,
    ! [N: $int] :
      ( $greatereq(N,2)
     => ( wrap(f(N)) = wrap(z) ) ) ).

