tff(left_type,type,
    left: $int > $o ).

tff(right_type,type,
    right: $int > $o ).

tff(left_base,axiom,
    left(0) ).

tff(right_base,axiom,
    right(0) ).

tff(left_step,axiom,
    ! [N: $int] :
      ( ( $greatereq(N,0)
        & left(N)
        & right(N) )
     => left($sum(N,1)) ) ).

tff(right_step,axiom,
    ! [N: $int] :
      ( ( $greatereq(N,0)
        & left(N)
        & right(N) )
     => right($sum(N,1)) ) ).

tff(goal,conjecture,
    ! [N: $int] :
      ( $greatereq(N,0)
     => ( left(N)
        & right(N) ) ) ).

