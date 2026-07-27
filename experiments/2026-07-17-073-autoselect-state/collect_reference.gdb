set pagination off
set confirm off
set print elements 0

start --help

set $init = (OrderParmsCell*) malloc(sizeof(OrderParmsCell))
call (void) memset($init, 90, sizeof(OrderParmsCell))
call (void) init_oparms($init)
printf "INIT,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d\n", $init->ordertype, $init->to_weight_gen, $init->to_prec_gen, $init->to_const_weight, $init->conj_only_mod, $init->conj_axiom_mod, $init->axiom_only_mod, $init->lit_cmp, $init->ho_order_kind, $init->db_w, $init->lam_w, $init->force_kbo_var_weight

set $mask = (OrderParmsCell*) malloc(sizeof(OrderParmsCell))
set $ordering = (OrderParmsCell*) malloc(sizeof(OrderParmsCell))
call (void) memset($mask, 0, sizeof(OrderParmsCell))
call (void) memset($ordering, 0, sizeof(OrderParmsCell))
set $mask->ordertype = NoOrdering
set $mask->to_weight_gen = WNoMethod
set $mask->to_prec_gen = PNoMethod
set $mask->to_const_weight = 0
set $ordering->ordertype = KBO
set $ordering->to_weight_gen = WMinMethod
set $ordering->to_prec_gen = PMinMethod
set $ordering->to_const_weight = 1

set $index = 0
printf "SEQ,%ld,%d,%d,%d,%ld\n", $index, $ordering->ordertype, $ordering->to_weight_gen, $ordering->to_prec_gen, $ordering->to_const_weight
while OrderNextOrdering($ordering, $mask)
  set $index = $index + 1
  printf "SEQ,%ld,%d,%d,%d,%ld\n", $index, $ordering->ordertype, $ordering->to_weight_gen, $ordering->to_prec_gen, $ordering->to_const_weight
end
printf "FINAL,%ld,%d,%d,%d,%ld\n", $index + 1, $ordering->ordertype, $ordering->to_weight_gen, $ordering->to_prec_gen, $ordering->to_const_weight

quit
