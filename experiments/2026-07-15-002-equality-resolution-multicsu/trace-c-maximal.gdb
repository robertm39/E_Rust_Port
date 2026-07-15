set pagination off
set confirm off
break EqnListMaximalLiterals if list != 0 && list->next != 0 && list->next->next != 0 && list->next->next->next == 0 && list->lterm->f_code == 28 && list->next->lterm->f_code == 28 && list->next->next->lterm->f_code == 17
commands
    silent
    set $a = list
    set $b = $a->next
    set $c = $b->next
    printf "codes=[(%ld,%ld),(%ld,%ld),(%ld,%ld)]\n", $a->lterm->f_code, $a->rterm->f_code, $b->lterm->f_code, $b->rterm->f_code, $c->lterm->f_code, $c->rterm->f_code
    printf "ocb sig_size=%ld var=%ld weights=[%ld,%ld,%ld,%ld,%ld] precedence=[%ld,%ld,%ld,%ld,%ld]\n", ocb->sig_size, ocb->var_weight, ocb->weights[17], ocb->weights[25], ocb->weights[26], ocb->weights[27], ocb->weights[28], ocb->prec_weights[17], ocb->prec_weights[25], ocb->prec_weights[26], ocb->prec_weights[27], ocb->prec_weights[28]
    print LiteralCompare(ocb, $a, $a)
    print LiteralCompare(ocb, $a, $b)
    print LiteralCompare(ocb, $a, $c)
    print LiteralCompare(ocb, $b, $a)
    print LiteralCompare(ocb, $b, $b)
    print LiteralCompare(ocb, $b, $c)
    print LiteralCompare(ocb, $c, $a)
    print LiteralCompare(ocb, $c, $b)
    print LiteralCompare(ocb, $c, $c)
    quit
end
run --unif-mode=multi --pattern-oracle=false --fixpoint-oracle=false --func-proj-limit=1 --imit-limit=1 --max-unifiers=4 --max-unif-steps=32 --output-level=0 --processed-clauses-limit=1 /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-002-equality-resolution-multicsu/input.p
