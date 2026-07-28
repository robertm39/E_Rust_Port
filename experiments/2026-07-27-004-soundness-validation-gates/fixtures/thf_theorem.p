%------------------------------------------------------------------------------
% Status : Theorem
%------------------------------------------------------------------------------
thf(person_type, type, person: $tType).
thf(p_type, type, p: person > $o).
thf(a_type, type, a: person).
thf(ax, axiom, p @ a).
thf(goal, conjecture, p @ a).
