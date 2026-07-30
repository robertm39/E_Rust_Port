tff(p_type,type,
    p: $int > $o ).

tff(goal,conjecture,
    ! [N: $int] :
      ( $greatereq(N,0)
     => p(N) ) ).

