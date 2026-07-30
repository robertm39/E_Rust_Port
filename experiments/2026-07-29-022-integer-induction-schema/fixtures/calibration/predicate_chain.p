tff(p_type,type,
    p: $int > $o ).

tff(base,axiom,
    p(0) ).

tff(step,axiom,
    ! [N: $int] :
      ( ( $greatereq(N,0)
        & p(N) )
     => p($sum(N,1)) ) ).

tff(goal,conjecture,
    ! [N: $int] :
      ( $greatereq(N,0)
     => p(N) ) ).
