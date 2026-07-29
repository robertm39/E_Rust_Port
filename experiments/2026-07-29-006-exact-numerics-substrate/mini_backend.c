#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "mini-mpq.h"

#define FNV_OFFSET UINT64_C(0xcbf29ce484222325)
#define FNV_PRIME UINT64_C(0x100000001b3)

typedef struct
{
    char *workload;
    mpq_t left;
    mpq_t right;
} case_t;

static void
fail(const char *message)
{
    fprintf(stderr, "error: %s\n", message);
    exit(EXIT_FAILURE);
}

static char *
copy_text(const char *text)
{
    char *copy = strdup(text);
    if (copy == NULL)
    {
        fail("out of memory");
    }
    return copy;
}

static void
set_rational(mpq_t destination, const char *numerator, const char *denominator)
{
    size_t length = strlen(numerator) + strlen(denominator) + 2;
    char *text = malloc(length);
    if (text == NULL)
    {
        fail("out of memory");
    }
    int written = snprintf(text, length, "%s/%s", numerator, denominator);
    if (written < 0 || (size_t) written >= length)
    {
        free(text);
        fail("cannot format rational");
    }
    if (mpq_set_str(destination, text, 10) != 0)
    {
        free(text);
        fail("cannot parse rational");
    }
    free(text);
    if (mpz_sgn(mpq_denref(destination)) == 0)
    {
        fail("zero denominator");
    }
    mpq_canonicalize(destination);
}

static case_t *
read_cases(const char *path, size_t *case_count)
{
    FILE *stream = fopen(path, "r");
    if (stream == NULL)
    {
        fail("cannot open vector file");
    }
    case_t *cases = NULL;
    size_t capacity = 0;
    size_t count = 0;
    char *line = NULL;
    size_t line_capacity = 0;
    while (getline(&line, &line_capacity, stream) >= 0)
    {
        size_t length = strlen(line);
        while (length > 0 && (line[length - 1] == '\n' || line[length - 1] == '\r'))
        {
            line[--length] = '\0';
        }
        if (length == 0 || line[0] == '#')
        {
            continue;
        }
        char *fields[5] = {0};
        char *save = NULL;
        char *token = strtok_r(line, "|", &save);
        size_t field_count = 0;
        while (token != NULL && field_count < 5)
        {
            fields[field_count++] = token;
            token = strtok_r(NULL, "|", &save);
        }
        if (field_count != 5 || token != NULL)
        {
            fail("vector line does not have five fields");
        }
        if (count == capacity)
        {
            size_t next_capacity = capacity == 0 ? 256 : capacity * 2;
            case_t *next = realloc(cases, next_capacity * sizeof(*cases));
            if (next == NULL)
            {
                fail("out of memory");
            }
            cases = next;
            capacity = next_capacity;
        }
        cases[count].workload = copy_text(fields[0]);
        mpq_init(cases[count].left);
        mpq_init(cases[count].right);
        set_rational(cases[count].left, fields[1], fields[2]);
        set_rational(cases[count].right, fields[3], fields[4]);
        if (mpq_sgn(cases[count].right) == 0)
        {
            fail("right operand is zero");
        }
        count++;
    }
    free(line);
    if (ferror(stream))
    {
        fclose(stream);
        fail("cannot read vector file");
    }
    fclose(stream);
    if (count == 0)
    {
        fail("vector file contains no cases");
    }
    *case_count = count;
    return cases;
}

static uint64_t
hash_text(uint64_t state, const char *text)
{
    const unsigned char *cursor = (const unsigned char *) text;
    while (*cursor != '\0')
    {
        state ^= *cursor++;
        state *= FNV_PRIME;
    }
    return state;
}

static uint64_t
hash_integer(uint64_t state, mpz_srcptr numerator, mpz_srcptr denominator)
{
    char *numerator_text = mpz_get_str(NULL, 10, numerator);
    char *denominator_text = mpz_get_str(NULL, 10, denominator);
    if (numerator_text == NULL || denominator_text == NULL)
    {
        free(numerator_text);
        free(denominator_text);
        fail("cannot serialize integer");
    }
    state = hash_text(state, numerator_text);
    state = hash_text(state, "/");
    state = hash_text(state, denominator_text);
    state = hash_text(state, "\n");
    free(numerator_text);
    free(denominator_text);
    return state;
}

static uint64_t
hash_rational(uint64_t state, mpq_srcptr value)
{
    return hash_integer(state, mpq_numref(value), mpq_denref(value));
}

static unsigned
iterations_for(const char *workload)
{
    if (strcmp(workload, "paper") == 0)
    {
        return 500;
    }
    if (strcmp(workload, "small") == 0)
    {
        return 80;
    }
    if (strcmp(workload, "medium") == 0)
    {
        return 12;
    }
    if (strcmp(workload, "large") == 0)
    {
        return 2;
    }
    fail("unknown workload");
    return 0;
}

static size_t
count_workload(const case_t *cases, size_t case_count, const char *workload)
{
    size_t count = 0;
    for (size_t index = 0; index < case_count; index++)
    {
        if (strcmp(cases[index].workload, workload) == 0)
        {
            count++;
        }
    }
    return count;
}

static uint64_t
correctness_digest(const case_t *cases, size_t case_count, const char *workload)
{
    mpq_t add;
    mpq_t subtract;
    mpq_t multiply;
    mpq_t divide;
    mpq_t integer_value;
    mpz_t integer;
    mpq_init(add);
    mpq_init(subtract);
    mpq_init(multiply);
    mpq_init(divide);
    mpq_init(integer_value);
    mpz_init(integer);
    uint64_t digest = FNV_OFFSET;
    for (size_t index = 0; index < case_count; index++)
    {
        if (strcmp(cases[index].workload, workload) != 0)
        {
            continue;
        }
        mpq_add(add, cases[index].left, cases[index].right);
        mpq_sub(subtract, cases[index].left, cases[index].right);
        mpq_mul(multiply, cases[index].left, cases[index].right);
        mpq_div(divide, cases[index].left, cases[index].right);
        digest = hash_rational(digest, cases[index].left);
        digest = hash_rational(digest, cases[index].right);
        digest = hash_rational(digest, add);
        digest = hash_rational(digest, subtract);
        digest = hash_rational(digest, multiply);
        digest = hash_rational(digest, divide);
        mpz_fdiv_q(integer, mpq_numref(cases[index].left), mpq_denref(cases[index].left));
        mpq_set_z(integer_value, integer);
        digest = hash_rational(digest, integer_value);
        mpz_cdiv_q(integer, mpq_numref(cases[index].left), mpq_denref(cases[index].left));
        mpq_set_z(integer_value, integer);
        digest = hash_rational(digest, integer_value);
        int ordering = mpq_cmp(cases[index].left, cases[index].right);
        mpq_set_si(integer_value, ordering < 0 ? -1 : ordering > 0 ? 1 : 0, 1);
        digest = hash_rational(digest, integer_value);
    }
    mpq_clear(add);
    mpq_clear(subtract);
    mpq_clear(multiply);
    mpq_clear(divide);
    mpq_clear(integer_value);
    mpz_clear(integer);
    return digest;
}

static uint64_t
elapsed_nanoseconds(struct timespec start, struct timespec end)
{
    uint64_t seconds = (uint64_t) (end.tv_sec - start.tv_sec);
    int64_t nanoseconds = end.tv_nsec - start.tv_nsec;
    if (nanoseconds < 0)
    {
        seconds--;
        nanoseconds += 1000000000;
    }
    return seconds * UINT64_C(1000000000) + (uint64_t) nanoseconds;
}

static uint64_t
timed_workload(
    const case_t *cases,
    size_t case_count,
    const char *workload,
    unsigned iterations,
    int64_t *sink_result)
{
    mpq_t add;
    mpq_t subtract;
    mpq_t multiply;
    mpq_t divide;
    mpz_t floor_value;
    mpz_t ceiling_value;
    mpq_init(add);
    mpq_init(subtract);
    mpq_init(multiply);
    mpq_init(divide);
    mpz_init(floor_value);
    mpz_init(ceiling_value);
    volatile int64_t sink = 0;
    struct timespec start;
    struct timespec end;
    if (clock_gettime(CLOCK_MONOTONIC, &start) != 0)
    {
        fail("clock_gettime failed");
    }
    for (unsigned iteration = 0; iteration < iterations; iteration++)
    {
        for (size_t index = 0; index < case_count; index++)
        {
            if (strcmp(cases[index].workload, workload) != 0)
            {
                continue;
            }
            mpq_add(add, cases[index].left, cases[index].right);
            mpq_sub(subtract, cases[index].left, cases[index].right);
            mpq_mul(multiply, cases[index].left, cases[index].right);
            mpq_div(divide, cases[index].left, cases[index].right);
            mpz_fdiv_q(
                floor_value,
                mpq_numref(cases[index].left),
                mpq_denref(cases[index].left));
            mpz_cdiv_q(
                ceiling_value,
                mpq_numref(cases[index].left),
                mpq_denref(cases[index].left));
            int comparison = mpq_cmp(add, subtract);
            sink += comparison < 0 ? -1 : comparison > 0 ? 1 : 0;
            sink += mpq_sgn(multiply) + mpq_sgn(divide);
            sink += mpz_sgn(floor_value) + mpz_sgn(ceiling_value);
        }
    }
    if (clock_gettime(CLOCK_MONOTONIC, &end) != 0)
    {
        fail("clock_gettime failed");
    }
    mpq_clear(add);
    mpq_clear(subtract);
    mpq_clear(multiply);
    mpq_clear(divide);
    mpz_clear(floor_value);
    mpz_clear(ceiling_value);
    *sink_result = sink;
    return elapsed_nanoseconds(start, end);
}

int
main(int argc, char **argv)
{
    if (argc != 2)
    {
        fail("usage: mini_backend VECTOR_FILE");
    }
    size_t case_count = 0;
    case_t *cases = read_cases(argv[1], &case_count);
    static const char *workloads[] = {"large", "medium", "paper", "small"};
    printf("{\"schema_version\":1,\"backend\":\"mini-gmp-6.3.0\",\"workloads\":[");
    for (size_t workload_index = 0; workload_index < 4; workload_index++)
    {
        const char *workload = workloads[workload_index];
        size_t count = count_workload(cases, case_count, workload);
        if (count == 0)
        {
            fail("missing workload");
        }
        unsigned iterations = iterations_for(workload);
        uint64_t digest = correctness_digest(cases, case_count, workload);
        int64_t sink = 0;
        uint64_t elapsed = timed_workload(
            cases,
            case_count,
            workload,
            iterations,
            &sink);
        if (workload_index != 0)
        {
            printf(",");
        }
        printf(
            "{\"name\":\"%s\",\"cases\":%zu,\"iterations\":%u,"
            "\"operations_per_case\":7,\"elapsed_ns\":%" PRIu64 ","
            "\"digest\":\"%016" PRIx64 "\",\"sink\":%" PRId64 "}",
            workload,
            count,
            iterations,
            elapsed,
            digest,
            sink);
    }
    printf("]}\n");
    for (size_t index = 0; index < case_count; index++)
    {
        free(cases[index].workload);
        mpq_clear(cases[index].left);
        mpq_clear(cases[index].right);
    }
    free(cases);
    return EXIT_SUCCESS;
}
