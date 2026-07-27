thf(animal_type, type, animal: $tType).
thf(cat_type, type, cat: animal).
thf(dog_type, type, dog: animal).
thf(a_type, type, a: $i).
thf(f_type, type, f: animal > animal).
thf(g_type, type, g: $i > $i).
thf(ax1, axiom, ((f @ cat) = dog)).
thf(ax2, axiom, ((f @ cat) = cat)).
thf(ax3, axiom, ((g @ a) = a)).
