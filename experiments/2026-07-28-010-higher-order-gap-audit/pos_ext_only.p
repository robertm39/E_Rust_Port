thf(person_type, type, person: $tType).
thf(f_type, type, f: person > person).
thf(g_type, type, g: person > person).
thf(p_type, type, p: $o).
thf(pointwise_or_p, axiom, ![X: person]: (((f @ X) = (g @ X)) | p)).
thf(not_p, axiom, ~p).
thf(functions_differ, axiom, f != g).
