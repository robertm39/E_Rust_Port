#ifndef UMLAUT_CADICAL_H
#define UMLAUT_CADICAL_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct UmlautCadical UmlautCadical;
typedef int (*UmlautCadicalTerminate)(void *);

const char *umlaut_cadical_signature(void);
UmlautCadical *umlaut_cadical_init(void);
void umlaut_cadical_release(UmlautCadical *);
const char *umlaut_cadical_last_error(const UmlautCadical *);

int umlaut_cadical_set_terminate(
    UmlautCadical *,
    void *,
    UmlautCadicalTerminate
);
int umlaut_cadical_add(UmlautCadical *, int);
int umlaut_cadical_assume(UmlautCadical *, int);
int umlaut_cadical_limit_decisions(UmlautCadical *, int);
int umlaut_cadical_solve(UmlautCadical *);
int umlaut_cadical_val(UmlautCadical *, int);
int umlaut_cadical_failed(UmlautCadical *, int);

int umlaut_cadical_trace_proof(UmlautCadical *, const char *);
int umlaut_cadical_conclude(UmlautCadical *);
int umlaut_cadical_close_proof(UmlautCadical *);

#ifdef __cplusplus
}
#endif

#endif
