tff(value_type,type,
    value: $tType ).

tff(zero_value_type,type,
    zero_value: value ).

tff(f_type,type,
    f: $int > value ).

tff(h_type,type,
    h: value > value ).

tff(base,axiom,
    f(0) = zero_value ).

tff(fixed,axiom,
    h(zero_value) = zero_value ).

tff(step,axiom,
    ! [N: $int] :
      ( $greatereq(N,0)
     => ( f($sum(N,1)) = h(f(N)) ) ) ).

tff(goal,conjecture,
    ! [N: $int] :
      ( $greatereq(N,0)
     => ( f(N) = zero_value ) ) ).

