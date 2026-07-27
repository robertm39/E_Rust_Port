tff(animal_type, type, animal: $tType).
tff(p_type, type, p: (animal * animal * animal) > $o).
tff(a_type, type, a: animal).
tff(b_type, type, b: animal).
tff(c_type, type, c: animal).
tff(variable_order, axiom, ![X:animal, Y:animal, Z:animal]: p(X, Y, Z)).
tff(variable_goal, conjecture, p(a, b, c)).
