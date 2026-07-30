tff(q_type,type,
    q: $int > $o ).

tff(base,axiom,
    q(3) ).

tff(step,axiom,
    ! [N: $int] :
      ( ( $greatereq(N,3)
        & q(N) )
     => q($sum(N,1)) ) ).

tff(goal,conjecture,
    ! [N: $int] :
      ( $lesseq(3,N)
     => q(N) ) ).

