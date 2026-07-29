tff(person_type, type, person: $tType).
tff(color_type, type, color: $tType).
tff(alice_type, type, alice: person).
tff(red_type, type, red: color).
tff(favorite_type, type, favorite: person > color).
tff(likes_type, type, likes: person * color > $o).
tff(favorite_red, axiom, favorite(alice) = red).
tff(alice_likes_favorite, axiom, likes(alice, favorite(alice))).
