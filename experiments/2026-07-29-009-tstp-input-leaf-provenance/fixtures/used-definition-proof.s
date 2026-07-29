% SZS status Unsatisfiable
% SZS output start CNFRefutation
cnf(q_source,axiom,q,file('used-definition-problem.p',q_source)).
cnf(mixed_source,axiom,(p|~q),file('used-definition-problem.p',mixed_source)).
cnf(not_p_source,axiom,~p,file('used-definition-problem.p',not_p_source)).
fof(test_definition,definition,(epred1_0<=>q),introduced(definition,[new_symbols(definition,[epred1_0])],[])).
cnf(test_split,plain,(epred1_0|~q),inference(split_equiv,[status(thm)],[test_definition])).
cnf(test_ep,plain,epred1_0,inference(cn,[status(thm)],[test_split,q_source])).
cnf(p_step,plain,p,inference(cn,[status(thm)],[mixed_source,q_source])).
cnf(false,plain,$false,inference(cn,[status(thm)],[p_step,not_p_source,test_ep]),['proof']).
% SZS output end CNFRefutation
