set pagination off
set breakpoint pending on
break che_tfidfweight.c:193
commands
  silent
  printf "C TF-IDF term code=%ld arity=%d tf_fact=%.17g repr=%p reprcode=%ld cell=%p cellkey=%ld cellval=%ld tf=%.9g df=%.9g docs=%ld idf=%.9g weight=%.9g\n", term->f_code, term->arity, data->tf_fact, repr, repr ? repr->f_code : -1, cell, cell ? cell->key : -1, cell ? cell->val1.i_val : -1, tf, df, data->documents->clause_count, idf, 1/(1+tfidf)
  continue
end
run
