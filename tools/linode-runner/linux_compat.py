#!/usr/bin/env python3
"""Build Linux E references and compare them with the native Linux Rust port.

This program is intentionally standard-library-only and runs on the ephemeral
Linode worker.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import random
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterable, Sequence


SZS_RE = re.compile(r"\bSZS\s+status\s+([^\s]+)", re.IGNORECASE)
SZS_OUTPUT_START_RE = re.compile(r"\bSZS\s+output\s+start\s+([^\s]+)", re.IGNORECASE)
SZS_OUTPUT_END_RE = re.compile(r"\bSZS\s+output\s+end\s+([^\s]+)", re.IGNORECASE)
EXPECTED_RE = re.compile(r"^%\s*Status\s*:\s*([^\s]+)", re.MULTILINE | re.IGNORECASE)
SATURATION_GENERATED_ID_RE = re.compile(r"\bc_\d+_\d+\b")
APP_ENCODE_TYPE_DECL_RE = re.compile(
    r"^tff\(typedecl\d+, type, type_(\d+): \$tType\)\.$"
)
VOLATILE_LINE_RE = re.compile(
    r"(?:User time|System time|Total time|Maximum resident|date|timestamp)\s*:",
    re.IGNORECASE,
)
PLATFORM_NAN_TOKEN = (
    r"(?:[-+]?nan(?:\([^)]*\))?|[-+]?1\.\#(?:ind|qnan|snan)\d*)"
)
PLATFORM_NAN_PERCENT_RE = re.compile(
    r"\bsuccesses,\s+"
    + PLATFORM_NAN_TOKEN
    + r"\s+percent\b",
    re.IGNORECASE,
)
PLATFORM_TERMPROPS_NAN_RE = re.compile(
    r"(?P<label>\b(?:ASize|ADepth):\s*)" + PLATFORM_NAN_TOKEN + r"(?=\s)",
    re.IGNORECASE,
)
PLATFORM_EPCLANALYSE_NAN_RE = re.compile(
    r"(?P<label>:)\s*" + PLATFORM_NAN_TOKEN + r"$",
    re.IGNORECASE,
)
CLASSIFY_LEGACY_FEATURE_SUFFIX_RE = re.compile(
    r"^(?P<prefix>.* : \(.*),\s*[-+]?\d+,\s*[-+]?\d+,\s*[^,]+,\s*[^,]+,"
    r"\s*(?:true|false),\s*(?:true|false)\s+\) : "
    r"(?P<class_prefix>[A-Z0-9-]{14})[A-Z0-9-]{7}$"
)
EPCLANALYSE_AVERAGE_PREFIXES = (
    "% Average number of ",
    "% ...in ",
    "% ...positive literals only",
    "% ...negative literals only",
)
PLATFORM_PROOFCHECK_TEMP_RE = re.compile(
    r"(?:(?:[A-Za-z]:)?[^ \t\r\n<>\"]*[\\/])?epr_[A-Za-z0-9]{6}"
)
LEGACY_SERVER_ACCEPTED_DESCRIPTOR_RE = re.compile(r"^Accepted [0-9]+$")
PLATFORM_ERROR_SUFFIXES = {
    "<OS ERROR: NOT FOUND>": (
        "No such file or directory",
        "The system cannot find the file specified. (os error 2)",
        "The system cannot find the path specified. (os error 3)",
    ),
    "<OS ERROR: BROKEN PIPE>": (
        "Broken pipe",
        "The pipe has been ended. (os error 109)",
        "The pipe is being closed. (os error 232)",
    ),
    "<C ERROR: RANGE>": (
        "Numerical result out of range",
        "Result too large",
    ),
}
FIXTURE_ARGUMENT_RE = re.compile(r"\{fixture:([^}]+)\}")
COMPANION_ARGUMENT_RE = re.compile(r"\{companion:([^}]+)\}")
TOOL_CASE_METADATA_KEYS = frozenset(
    {
        "fixture_files",
        "isolated_workdir",
        "workdir_files",
        "workdir_directories",
        "output_files",
        "output_absent_files",
        "output_directories",
        "normalize_legacy_classify_feature_suffix",
        "expected_mismatches",
        "reference_mode",
    }
)
TOOL_COMPARISON_MISMATCH_FIELDS = frozenset(
    {
        "exit_code",
        "timed_out",
        "status",
        "shape",
        "normalized_stdout",
        "normalized_stderr",
        "output_files",
        "output_absent_files",
        "output_directories",
    }
)
MAIN_COMPARISON_EXPECTED_MISMATCHES = {
    ("ho", "sledgehammer.p"): ("normalized_stdout",),
}
MAIN_COMPARISON_RESOURCE_STRESS_CASES = frozenset(
    {
        "BOO020-1.p",
        "SWV851-1.p",
    }
)
MAIN_COMPARISON_MINIMUM_CPU_LIMITS = {
    "HEN011-2.p": 90,
}
PROCESS_TIMEOUT_GRACE_SECONDS = 30
PROBLEM_SUFFIXES = {".p", ".lop"}
DEFAULT_DISTRO = "Ubuntu-24.04"
REFERENCE_TOOL_BINARIES = {
    "CSSCPA_filter": "EXTERNAL/CSSCPA_filter",
    "checkproof": "PROVER/checkproof",
    "classify_problem": "PROVER/classify_problem",
    "direct_examples": "PROVER/direct_examples",
    "e_axfilter": "PROVER/e_axfilter",
    "e_client": "PROVER/e_client",
    "e_deduction_server": "PROVER/e_deduction_server",
    "e_ltb_runner": "PROVER/e_ltb_runner",
    "e_server": "PROVER/e_server",
    "e_stratpar": "PROVER/e_stratpar",
    "edpll": "PROVER/edpll",
    "eground": "PROVER/eground",
    "ekb_create": "PROVER/ekb_create",
    "ekb_delete": "PROVER/ekb_delete",
    "ekb_ginsert": "PROVER/ekb_ginsert",
    "ekb_insert": "PROVER/ekb_insert",
    "enormalizer": "PROVER/enormalizer",
    "epatternize": "PROVER/epatternize",
    "epclanalyse": "PROVER/epclanalyse",
    "epclextract": "PROVER/epclextract",
    "epcllemma": "PROVER/epcllemma",
    "ex_commandline": "SIMPLE_APPS/ex_commandline",
    "term2dag": "SIMPLE_APPS/term2dag",
    "termprops": "PROVER/termprops",
    "tsm_classify": "PROVER/tsm_classify",
}
ARCHIVED_REFERENCE_TOOL_LINKS = {
    "termprops": (
        ("make", "termprops.o"),
        (
            "cc",
            "-o",
            "termprops",
            "termprops.o",
            "../lib/TERMS.a",
            "../lib/CLAUSES.a",
            "../lib/ORDERINGS.a",
            "../lib/TERMS.a",
            "../lib/INOUT.a",
            "../lib/BASICS.a",
            "../lib/CONTRIB.a",
            "-lm",
        ),
    ),
    "tsm_classify": (
        ("make", "tsm_classify.o"),
        (
            "cc",
            "-o",
            "tsm_classify",
            "tsm_classify.o",
            "../lib/LEARN.a",
            "../lib/CLAUSES.a",
            "../lib/ORDERINGS.a",
            "../lib/TERMS.a",
            "../lib/INOUT.a",
            "../lib/BASICS.a",
            "../lib/CONTRIB.a",
            "-lm",
        ),
    ),
}
ARCHIVED_REFERENCE_TOOL_SOURCE_PATCHES = {
    "termprops": (
        (
            Path("PROVER/termprops.c"),
            "ProblemType problemType  = PROBLEM_NOT_INIT;",
            "/* problemType is provided by BASICS.a in current upstream. */",
        ),
        (
            Path("PROVER/termprops.c"),
            "CreateScanner(StreamTypeFile, state->argv[i], true, NULL);",
            "CreateScanner(StreamTypeFile, state->argv[i], true, NULL, true);",
        ),
    ),
    "tsm_classify": (
        (
            Path("PROVER/tsm_classify.c"),
            "ProblemType problemType  = PROBLEM_NOT_INIT;",
            "/* problemType is provided by BASICS.a in current upstream. */",
        ),
        (
            Path("PROVER/tsm_classify.c"),
            "CreateScanner(StreamTypeFile, infile, true, NULL);",
            "CreateScanner(StreamTypeFile, infile, true, NULL, true);",
        ),
    ),
}
DEFAULT_TOOL_ARGUMENT_CASES = (("--help",),)
VERSIONED_REFERENCE_TOOLS = frozenset(REFERENCE_TOOL_BINARIES) - {
    "ex_commandline",
    "term2dag",
    "termprops",
}
DIRECT_EXAMPLES_BRANCHING_PROTOCOL = (
    "1 : : [++p(X)] : initial\n"
    "2 : : [++q(Y)] : initial\n"
    "3 : : [++r(X,Y)] : pm(1,2)\n"
    "4 : : [++s(X)] : 1\n"
    "5 : : [++t(Y)] : 2\n"
    "6 : : [++u(X,Y)] : pm(3,4)\n"
    "7 : : [++v(X)] : 6\n"
    "8 : : [++w(Y)] : 5\n"
    "9 : : [++x(X,Y)] : pm(7,8)\n"
    "10 : : [] : 9 : 'final'\n"
    "11 : : [++n(a)] : 4\n"
    "12 : : [++m(a)] : 11\n"
)
TSM_RECURSIVE_CORPUS = (
    "Training:\n"
    "a : 1:(1,-1).\n"
    "b : 2:(1,1).\n"
    "f(a) : 1:(1,-1).\n"
    "f(b) : 2:(1,1).\n"
    "g(a,b) : 1:(1,-1).\n"
    "g(b,a) : 2:(1,1).\n"
    "h(f(a),g(a,b)) : 1:(1,-1).\n"
    "h(f(b),g(b,a)) : 2:(1,1).\n"
    "f(g(a,b)) : 1:(1,-1).\n"
    "f(g(b,a)) : 2:(1,1).\n"
    "g(f(a),f(b)) : 1:(1,-1).\n"
    "g(f(b),f(a)) : 2:(1,1).\n"
    ".\n"
    "Test:\n"
    "a : 1:(1,-1).\n"
    "b : 2:(1,1).\n"
    "f(a) : 1:(1,-1).\n"
    "f(b) : 2:(1,1).\n"
    "g(a,b) : 1:(1,-1).\n"
    "g(b,a) : 2:(1,1).\n"
    "h(f(a),g(a,b)) : 1:(1,-1).\n"
    "h(f(b),g(b,a)) : 2:(1,1).\n"
    "f(h(f(a),g(a,b))) : 1:(1,-1).\n"
    "f(h(f(b),g(b,a))) : 2:(1,1).\n"
    "g(g(a,b),f(a)) : 1:(1,-1).\n"
    "g(g(b,a),f(b)) : 2:(1,1).\n"
    ".\n"
)


def csscpa_large_stateful_corpus() -> str:
    """Build a deterministic corpus spanning every CSSCPA clause outcome."""

    lines = ["output_level 0", "state:"]
    for index in range(24):
        source = 2 + index % 14
        lines.append(
            f"accept from {source}: "
            f"cnf(csscpa_seed_{index},axiom,csscpa_seed_{index}(a))."
        )
    for index in range(8):
        lines.append(
            f"accept: cnf(csscpa_negative_{index},axiom,~csscpa_negative_{index}(a))."
        )
    for index in range(8):
        lines.append(
            f"accept: cnf(csscpa_wide_{index},axiom,"
            f"(csscpa_wide_{index}(a)|csscpa_side_{index}(a)))."
        )

    lines.extend(("output_level 1", "state:"))
    for index in range(12):
        lines.append(
            "check improve(0.0,0.0): "
            f"cnf(csscpa_subsumed_{index},axiom,"
            f"(csscpa_seed_{index}(a)|csscpa_extra_{index}(a)))."
        )
    for index in range(4):
        lines.append(
            f"check: cnf(csscpa_tautology_{index},axiom,"
            f"(csscpa_taut_{index}(a)|~csscpa_taut_{index}(a)))."
        )
    for index in range(8):
        lines.append(
            "check improve(0.0,1.0): "
            f"cnf(csscpa_improved_{index},axiom,csscpa_wide_{index}(a))."
        )
    for index in range(4):
        lines.append(
            "check improve(1.0,1.0): "
            f"cnf(csscpa_contradiction_{index},axiom,csscpa_negative_{index}(a))."
        )
    for index in range(4):
        lines.append(
            "check improve(1.0,1.0): "
            f"cnf(csscpa_weighty_{index},axiom,"
            f"(csscpa_heavy_{index}(f(a))|csscpa_other_{index}(g(a))))."
        )
    lines.extend(
        (
            "Please process clauses now, I beg you, great shining CSSCPA,",
            "wonder of the world, most beautiful program ever written.",
            "state:",
        )
    )
    return "\n".join(lines) + "\n"


CSSCPA_LARGE_STATEFUL_CORPUS = csscpa_large_stateful_corpus()
EPCLLEMMA_FORMULA_PROTOCOL = "1 : : p(a) : initial\n2 : : q(a) : 1\n"
EPCLLEMMA_LARGE_PROTOCOL = "".join(
    f"{step_id} : : [++p(a)] : initial\n" for step_id in range(1, 1_011)
)
TOOL_FUNCTIONAL_CASES = {
    "CSSCPA_filter": (
        (
            "silent-accept",
            ("--silent",),
            "accept: cnf(csscpa_unit,axiom,p(a)).\n",
        ),
        (
            "trace-state-check",
            (),
            (
                "output_level 0\n"
                "state:\n"
                "output_level 1\n"
                "accept from 2: cnf(csscpa_unit,axiom,p(a)).\n"
                "check improve(0.0,0.0): "
                "cnf(csscpa_candidate,axiom,(p(a)|q(a))).\n"
                "Please process clauses now, I beg you, great shining CSSCPA,\n"
                "wonder of the world, most beautiful program ever written.\n"
                "state:\n"
            ),
        ),
        (
            "large-stateful-corpus",
            (),
            CSSCPA_LARGE_STATEFUL_CORPUS,
            {"expected_mismatches": ("normalized_stdout",)},
        ),
        (
            "missing-input",
            ("missing-csscpa-input.csscpa",),
            None,
            {"isolated_workdir": True},
        ),
    ),
    "checkproof": (
        (
            "assumption-only",
            (),
            "1 : : [++p(a)] : initial\n",
        ),
        (
            "setheo-release-failure",
            ("--prover-type=scheme-setheo",),
            (
                "1 : : [++p(a)] : initial\n"
                "2 : : [++q(a)] : 1\n"
                "3 : : [++r(a)] : split(2)\n"
            ),
        ),
        (
            "real-e-single-percent-marker-success",
            ("--output-level=3", "--executable={companion:eprover}"),
            "1 : : [++p(a)] : initial\n2 : : [++p(a)] : 1\n",
            {"expected_mismatches": ["normalized_stdout"]},
        ),
        (
            "real-e-failure",
            ('--executable="{companion:eprover}"',),
            "1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1\n",
        ),
        (
            "e-single-percent-marker-success",
            ("--output-level=3", "--executable=echo % Proof found!"),
            (
                "1 : : [++p(X),--q(f(X))] : initial\n"
                "2 : : [++r(X),--s(X)] : 1\n"
            ),
            {"expected_mismatches": ["normalized_stdout"]},
        ),
        (
            "e-double-percent-marker-success",
            ("--output-level=3", "--executable=echo %% Proof found!"),
            "1 : : [++p(a)] : initial\n2 : : [++p(a)] : 1\n",
        ),
        (
            "e-shell-failure",
            ("--output-level=3", "--executable=echo NO-PROOF"),
            (
                "1 : : [++p(X),--q(f(X))] : initial\n"
                "2 : : [++r(X),--s(X)] : 1\n"
            ),
        ),
        (
            "otter-shell-failure",
            ("--prover-type=Otter", "--executable=echo NO-PROOF"),
            (
                "1 : : [++p(X),--q(f(X))] : initial\n"
                "2 : : [++r(X),--s(X)] : 1\n"
            ),
        ),
        (
            "otter-shell-success",
            (
                "--prover-type=Otter",
                "--executable=echo -------- PROOF --------",
            ),
            "1 : : [++p(a)] : initial\n2 : : [++p(a)] : 1\n",
        ),
        (
            "spass-shell-failure",
            ("--prover-type=SPASS", "--executable=echo NO-PROOF"),
            (
                "1 : : [++p(X),--q(f(X))] : initial\n"
                "2 : : [++r(X),--s(X)] : 1\n"
            ),
        ),
        (
            "spass-shell-success",
            ("--prover-type=SPASS", "--executable=echo Proof found."),
            "1 : : [++p(a)] : initial\n2 : : [++p(a)] : 1\n",
        ),
        (
            "fof-warning-setheo",
            ("--prover-type=scheme-setheo",),
            (
                "1 : : p(a) : initial\n"
                "2 : : [++q(a)] : 1\n"
                "3 : : r(a) : 2\n"
            ),
        ),
        (
            "shell-step-rejection",
            (),
            "1 : : : initial\n",
        ),
        (
            "missing-input",
            ("missing-checkproof-input.pcl",),
            None,
            {"isolated_workdir": True},
        ),
    ),
    "classify_problem": (
        (
            "parse-features-standard",
            ("--parse-features",),
            (
                "prob : "
                "(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0.25,0.75,16,17,18,19,20): "
                "UHSMG\n"
            ),
            {"normalize_legacy_classify_feature_suffix": True},
        ),
        (
            "parse-features-raw",
            ("--parse-features", "--raw-class"),
            (
                "raw : (1,2,3,4,5,6,7,8,0.125,9,true,2,0,false): "
                "FSSMMLLCCSSNAA\n"
            ),
        ),
        (
            "parse-features-missing-colon",
            ("--parse-features",),
            "broken\n",
        ),
        (
            "parse-features-short-class",
            ("--parse-features",),
            (
                "prob : "
                "(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0.25,0.75,16,17,18,19,20): "
                "H\n"
            ),
        ),
        (
            "parse-features-raw-short-class",
            ("--parse-features", "--raw-class"),
            "raw : (1,2,3,4,5,6,7,8,0.125,9,true,2,0,false): short\n",
        ),
        (
            "parse-features-output-file",
            ("--parse-features", "-o", "features.out"),
            (
                "fileprob : "
                "(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0.25,0.75,16,17,18,19,20): "
                "UHSMG\n"
            ),
            {
                "isolated_workdir": True,
                "output_files": ("features.out",),
                "normalize_legacy_classify_feature_suffix": True,
            },
        ),
        (
            "raw-lop",
            ("--raw-class", "--lop-in"),
            "p(a).\nq(a).\n",
        ),
        (
            "old-tptp-records",
            ("--tptp-in",),
            (
                "input_formula(f1,axiom,p(a)).\n"
                "input_clause(c1,axiom,[++p(a)]).\n"
            ),
        ),
        (
            "raw-fof-definition-conjecture",
            ("--raw-class", "--tstp-format"),
            (
                "fof(definition,axiom,![X]:(f(X)=X)).\n"
                "fof(goal,conjecture,?[X]:p(f(X))).\n"
            ),
        ),
        (
            "standard-fof-definition-conjecture",
            ("--tstp-format",),
            (
                "fof(definition,axiom,![X]:(f(X)=X)).\n"
                "fof(goal,conjecture,?[X]:p(f(X))).\n"
            ),
        ),
        (
            "tstp-first-order-record-mix",
            ("--tstp-format",),
            (
                "tff(person_type,type,person:$tType).\n"
                "tff(a_type,type,a:person).\n"
                "tff(p_type,type,p:person>$o).\n"
                "fof(f1,axiom,p(a)).\n"
                "tcf(c1,axiom,![X:person]:p(X)).\n"
                "cnf(c2,axiom,p(a)).\n"
            ),
        ),
        (
            "fool-term-let",
            ("--tstp-format",),
            (
                "tff(a_type,type,a:$i).\n"
                "tff(p_type,type,p:$i>$o).\n"
                "fof(fool_owner,axiom,p($let(f:$i,f:=a,f))).\n"
            ),
            {
                "reference_mode": "ho",
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stdout",
                    "normalized_stderr",
                ),
            },
        ),
        (
            "raw-thf",
            ("--raw-class", "--tstp-format"),
            (
                "thf(person_type,type,person:$tType).\n"
                "thf(a_type,type,a:person).\n"
                "thf(p_type,type,p:person>$o).\n"
                "thf(fact,axiom,p@a).\n"
            ),
            {"reference_mode": "ho"},
        ),
        (
            "specsig-mixed-arities",
            ("--tstp-format", "--specsig"),
            (
                "cnf(c1,axiom,(p(a)|q(f(a),X))).\n"
                "cnf(c2,negated_conjecture,(~p(b)|f(Y)=Y)).\n"
            ),
        ),
        (
            "tptp-header-mixed-shape",
            ("--tstp-format", "--generate-tptp-header"),
            (
                "cnf(c1,axiom,(p(a)|q(f(a),X))).\n"
                "cnf(c2,negated_conjecture,(~p(b)|f(Y)=Y)).\n"
            ),
        ),
        (
            "include-selector",
            ("main.p",),
            None,
            {
                "workdir_files": {
                    "main.p": "include('selected.p',[selected]).\nfof(local,axiom,q(a)).\n",
                    "selected.p": (
                        "fof(selected,axiom,p(a)).\n"
                        "fof(dropped,axiom,r(a)).\n"
                    ),
                },
            },
        ),
        (
            "merged-positive-cnf",
            ("--tstp-format", "--merged-classification=2"),
            "cnf(c1,axiom,p(a)).\n",
        ),
        (
            "merged-positive-fool",
            ("--tstp-format", "--merged-classification=2"),
            (
                "tff(a_type,type,a:$i).\n"
                "tff(p_type,type,p:$i>$o).\n"
                "fof(fool_owner,axiom,p($let(f:$i,f:=a,f))).\n"
            ),
            {
                "reference_mode": "ho",
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stdout",
                    "normalized_stderr",
                ),
            },
        ),
        (
            "merged-zero-fast-child",
            ("--tstp-format", "--merged-classification=0"),
            "cnf(c1,axiom,p(a)).\n",
        ),
        (
            "merged-negative-unbounded",
            ("--tstp-format", "--merged-classification=-2"),
            "cnf(c1,axiom,p(a)).\n",
        ),
        (
            "merged-minus-one-standard",
            ("--tstp-format", "--merged-classification=-1"),
            "cnf(c1,axiom,p(a)).\n",
        ),
        (
            "merged-positive-thf",
            ("--tstp-format", "--merged-classification=2"),
            (
                "thf(a_type,type,a:$i).\n"
                "thf(p_type,type,p:$i>$o).\n"
                "thf(fact,axiom,p@a).\n"
            ),
            {"reference_mode": "ho"},
        ),
        (
            "missing-feature-input",
            ("--parse-features", "missing-classify-features.txt"),
            None,
            {"isolated_workdir": True},
        ),
        (
            "missing-real-input",
            ("--tstp-format", "missing-classify-problem.p"),
            None,
            {"isolated_workdir": True},
        ),
        (
            "missing-output-parent",
            ("--parse-features", "-o", "missing/features.out"),
            (
                "prob : "
                "(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0.25,0.75,16,17,18,19,20): "
                "UHSMG\n"
            ),
            {
                "isolated_workdir": True,
                "output_absent_files": ("missing/features.out",),
            },
        ),
    ),
    "direct_examples": (
        (
            "stdin-basic",
            (),
            "1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1\n",
        ),
        (
            "branching-protocol",
            ("--negative-example-proportion=1.5", "--negative-example-number=12"),
            DIRECT_EXAMPLES_BRANCHING_PROTOCOL,
        ),
        (
            "missing-input",
            ("missing-learning-input.pcl",),
            None,
            {"isolated_workdir": True},
        ),
    ),
    "e_axfilter": (
        ("dump-filter-stdout", ("--dump-filter", "-o", "-"), None),
        (
            "tstp-threshold-file",
            ("--tstp-in", "-f", "filters.axf", "-o", "global.out", "problem.p"),
            None,
            {
                "workdir_files": {
                    "filters.axf": "tiny=Threshold(10000)\n",
                    "problem.p": "fof(a, axiom, p(a)).\n",
                },
                "output_files": ("global.out", "problem_tiny.p"),
                # The authoritative optimized C binary aborts after producing
                # partial output; the memory-safe Rust implementation must not
                # reproduce that double-free behavior.
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                    "output_files",
                ),
            },
        ),
        (
            "tstp-gsine-formulas",
            ("--tstp-in", "-f", "filters.axf", "-o", "global.out", "problem.p"),
            None,
            {
                "workdir_files": {
                    "filters.axf": (
                        "formulas=GSinE(CountTerms, ,false,10.0,100,100,10000,1.0)\n"
                    ),
                    "problem.p": (
                        "fof(goal, conjecture, p(goal_a)).\n"
                        "fof(link1, axiom, (p(goal_a) => q(link_b))).\n"
                        "fof(link2, axiom, (q(link_b) => s(link_c))).\n"
                        "fof(link3, axiom, (s(link_c) => t(link_d))).\n"
                        "fof(far1, axiom, r(far_c)).\n"
                        "fof(far2, axiom, u(far_d)).\n"
                    ),
                },
                "output_files": ("global.out", "problem_formulas.p"),
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                    "output_files",
                ),
            },
        ),
        (
            "tstp-lambda-def-formulas",
            ("--tstp-in", "-f", "filters.axf", "-o", "global.out", "problem.p"),
            None,
            {
                "workdir_files": {
                    "filters.axf": "defs=LambdaDef\n",
                    "problem.p": (
                        "thf(person_type, type, person: $tType).\n"
                        "thf(a_type, type, a: person).\n"
                        "thf(p_type, type, p: person > $o).\n"
                        "thf(q_type, type, q: person > $o).\n"
                        "thf(r_type, type, r: person > $o).\n"
                        "thf(lambda_def1, definition, p = (^[X: person]: (q @ X))).\n"
                        "thf(lambda_def2, definition, q = (^[X: person]: (r @ X))).\n"
                        "thf(goal, conjecture, p @ a).\n"
                        "thf(hyp, hypothesis, q @ a).\n"
                        "thf(question, question, r @ a).\n"
                        "thf(far, axiom, r @ a).\n"
                    ),
                },
                "output_files": ("global.out", "problem_defs.p"),
                "reference_mode": "ho",
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                    "output_files",
                ),
            },
        ),
        (
            "tstp-seeded-all-methods",
            (
                "--tstp-in",
                "-f",
                "filters.axf",
                "--seed-method=lda",
                "--seeds=p",
                "-o",
                "global.out",
                "problem.p",
            ),
            None,
            {
                "workdir_files": {
                    "filters.axf": (
                        "seed=GSinE(CountTerms,hypos,false,10.0,100,100,10000,1.0)\n"
                    ),
                    "problem.p": (
                        "fof(seed_small, axiom, p(a)).\n"
                        "fof(seed_large, axiom, p(f(g(a)))).\n"
                        "fof(other, axiom, q(b)).\n"
                    ),
                },
                "output_files": (
                    "global.out",
                    "problem_SA_P1_24_seed.p",
                    "problem_SL_P1_24_seed.p",
                    "problem_SD_P1_24_seed.p",
                ),
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stdout",
                    "normalized_stderr",
                    "output_files",
                ),
            },
        ),
        (
            "output-open-missing-parent",
            ("-o", "missing/global.out", "problem.p"),
            None,
            {"workdir_files": {"problem.p": "fof(a, axiom, p(a)).\n"}},
        ),
        (
            "filter-open-missing",
            ("-f", "missing.axf", "problem.p"),
            None,
            {"workdir_files": {"problem.p": "fof(a, axiom, p(a)).\n"}},
        ),
    ),
    "e_client": (
        ("invalid-port", ("--port=70000",), None),
    ),
    "e_deduction_server": (
        ("stdout-unimplemented", (), None),
    ),
    "e_ltb_runner": (
        ("usage-missing-spec", (), None),
    ),
    "e_server": (
        ("usage-missing-domain", (), None),
    ),
    "ekb_delete": (
        (
            "drop-example",
            ("--knowledge-base=kb", "drop"),
            None,
            {
                "workdir_files": {
                    "kb/problems": (
                        "% Example names and features. \n"
                        "1: \"drop\"\n"
                        "PA: () FA: () "
                        "(1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n"
                        "2: \"keep\"\n"
                        "PA: () FA: () "
                        "(2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n"
                    ),
                    "kb/clausepatterns": (
                        "% Individual annotated patterns. \n"
                        "p(a) : 1:(1,0,0,0,0,0,0),2:(1,0,0,0,0,0,0).\n"
                        "q(a) : 1:(1,0,0,0,0,0,0).\n"
                        "r(a) : 2:(1,0,0,0,0,0,0).\n"
                    ),
                    "kb/FILES/drop": "drop problem",
                    "kb/FILES/keep": "keep problem",
                },
                "workdir_directories": ("kb/FILES",),
                "output_files": (
                    "kb/FILES/keep",
                    "kb/problems",
                    "kb/clausepatterns",
                ),
                "output_absent_files": ("kb/FILES/drop",),
                # The authoritative optimized C binary aborts while updating
                # this valid knowledge base; Rust completes the update safely.
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                ),
            },
        ),
        (
            "drop-middle-example",
            ("--knowledge-base=kb", "middle"),
            None,
            {
                "workdir_files": {
                    "kb/problems": (
                        "% Example names and features. \n"
                        "1: \"one\"\n"
                        "PA: () FA: () "
                        "(1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n"
                        "2: \"middle\"\n"
                        "PA: () FA: () "
                        "(2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n"
                        "3: \"three\"\n"
                        "PA: () FA: () "
                        "(3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n"
                        "4: \"four\"\n"
                        "PA: () FA: () "
                        "(4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n"
                    ),
                    "kb/clausepatterns": (
                        "% Individual annotated patterns. \n"
                        "p(a) : 1:(1,0,0,0,0,0,0),2:(1,0,0,0,0,0,0),"
                        "3:(1,0,0,0,0,0,0),4:(1,0,0,0,0,0,0).\n"
                        "q(a) : 2:(1,0,0,0,0,0,0).\n"
                        "r(a) : 1:(1,0,0,0,0,0,0),4:(1,0,0,0,0,0,0).\n"
                        "s(a) : 3:(1,0,0,0,0,0,0).\n"
                    ),
                    "kb/FILES/one": "one problem",
                    "kb/FILES/middle": "middle problem",
                    "kb/FILES/three": "three problem",
                    "kb/FILES/four": "four problem",
                },
                "workdir_directories": ("kb/FILES",),
                "output_files": (
                    "kb/FILES/one",
                    "kb/FILES/three",
                    "kb/FILES/four",
                    "kb/problems",
                    "kb/clausepatterns",
                ),
                "output_absent_files": ("kb/FILES/middle",),
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                ),
            },
        ),
    ),
    "ekb_ginsert": (
        (
            "stdin-protocol",
            ("--knowledge-base=kb",),
            (
                "1 : : [++p(a)] : initial : 'proof'\n"
                "2 : : [++q(a)] : initial\n"
                "3 : : [++r(a)] : 2\n"
            ),
            {
                "workdir_files": {
                    "kb/description": (
                        "% E theorem prover knowledge base description\n"
                        "Version     : \"0.20dev\"\n"
                        "NegProp     : 1.000000  % Negative example proportion "
                        "(successful proof search)\n"
                        "FailExamples:        2  % Number of clauses from a failed proof search\n"
                    ),
                    "kb/signature": "",
                    "kb/problems": "",
                    "kb/clausepatterns": "",
                },
                "workdir_directories": ("kb/FILES",),
                "output_files": (
                    "kb/FILES/__problem__1",
                    "kb/problems",
                    "kb/clausepatterns",
                ),
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                    "output_files",
                ),
            },
        ),
    ),
    "ekb_create": (
        (
            "empty-kb-files",
            (
                "--negative-example-number=7",
                "--negative-example-proportion=0.5",
                "kb",
            ),
            None,
            {
                "output_files": (
                    "kb/description",
                    "kb/signature",
                    "kb/problems",
                    "kb/clausepatterns",
                ),
                "output_directories": ("kb/FILES",),
            },
        ),
    ),
    "ekb_insert": (
        (
            "stdin-example",
            ("--knowledge-base=kb",),
            "a=b.\n.\n0:(0): a=b.\n",
            {
                "workdir_files": {
                    "kb/signature": "",
                    "kb/problems": "",
                    "kb/clausepatterns": "",
                },
                "workdir_directories": ("kb/FILES",),
                "output_files": (
                    "kb/FILES/__problem__1",
                    "kb/problems",
                    "kb/clausepatterns",
                ),
                # The authoritative optimized C binary aborts on this valid
                # insertion after corrupting its heap; Rust completes safely.
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                ),
            },
        ),
    ),
    "e_stratpar": (
        ("usage-missing-problem", (), None),
    ),
    "edpll": (
        ("lop-basic", ("--dimacs",), "p <- q. r <- r."),
        ("tptp-input-clause", ("--tptp-in",), "input_clause(c_0_1,axiom,[++p,--q])."),
        ("contradictory-units-no-solver", (), "p.\n<- p.\n"),
        ("trailing-non-clause", (), "p. ,\n"),
        (
            "output-file",
            ("-o", "trace.out"),
            "p.\nq <- r.\n",
            {
                "isolated_workdir": True,
                "output_files": ("trace.out",),
            },
        ),
        ("malformed-term-after-prefix", (), "p.\nq(f(a).\n"),
        ("malformed-equation", (), "p(a)=.\n"),
        ("empty-procedural-tail", (), "p :- .\n"),
        (
            "resource-options-success",
            ("--cpu-limit=30", "--soft-cpu-limit=20", "--memory-limit=0"),
            "p.\n",
        ),
        (
            "invalid-hard-after-soft",
            ("--soft-cpu-limit=10", "--cpu-limit=10"),
            "p.\n",
        ),
        (
            "invalid-soft-after-hard",
            ("--cpu-limit=10", "--soft-cpu-limit=10"),
            "p.\n",
        ),
        (
            "missing-input",
            ("missing-edpll-input.lop",),
            None,
            {"isolated_workdir": True},
        ),
        (
            "missing-output-parent",
            ("-o", "missing/trace.out"),
            "p.\n",
            {
                "isolated_workdir": True,
                "output_absent_files": ("missing/trace.out",),
            },
        ),
    ),
    "eground": (
        ("lop-basic", ("--lop-in", "--silent"), "p(a).\n"),
        (
            "lop-non-unit-output",
            ("--lop-in", "--silent"),
            "p(a);q(a)<-r(a).\n",
        ),
        (
            "tptp-non-unit-output",
            ("--lop-in", "--tptp-out", "--silent"),
            "p(a);q(a)<-r(a).\n",
        ),
        (
            "tstp-non-unit-output",
            ("--lop-in", "--tstp-out", "--silent"),
            "p(a);q(a)<-r(a).\n",
        ),
        (
            "auto-tstp-non-unit-output",
            ("--silent",),
            "cnf(ax,axiom,(p(a)|q(a))).\n",
        ),
        (
            "tstp-formula-ground",
            ("--tstp-format", "--silent"),
            "fof(ax,axiom,p(a)).\n",
        ),
        (
            "selected-include",
            ("--tstp-in", "--silent", "main.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "main.p": "include('inc.p',[selected]).\n",
                    "inc.p": (
                        "fof(selected,axiom,p(a)).\n"
                        "fof(skipped,axiom,q(a)).\n"
                    ),
                },
            },
        ),
        (
            "nested-selected-include",
            ("--tstp-in", "--silent", "main.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "main.p": "include('child.p',[selected]).\n",
                    "child.p": (
                        "include('grand.p').\n"
                        "fof(unselected,axiom,q(a)).\n"
                    ),
                    "grand.p": "fof(selected,axiom,p(a)).\n",
                },
            },
        ),
        (
            "verbose-conjecture-progress",
            ("--tstp-in", "--verbose=1", "--silent", "--suppress-result"),
            "fof(goal,conjecture,p(a)).\n",
        ),
        (
            "dimacs-output-stream-split",
            ("--tstp-in", "--dimacs", "-o", "ground.cnf"),
            "fof(ax,axiom,(p(a)|q(a))).\n",
            {
                "isolated_workdir": True,
                "output_files": ("ground.cnf",),
            },
        ),
        ("malformed-term", ("--lop-in",), "p(f(a).\n"),
        ("trailing-token", ("--lop-in",), "p(a). ,\n"),
        ("non-ground-infinite-universe", ("--lop-in",), "p(f(X)).\n"),
        (
            "give-up-estimate",
            ("--lop-in", "--silent", "--give-up=1"),
            "p(a).\np(b).\nq(X).\n",
        ),
        (
            "constrained-give-up-estimate",
            ("--lop-in", "--silent", "--constraints", "--give-up=1"),
            "p(a).\np(b).\nq(X).\n",
        ),
        (
            "resource-options-success",
            (
                "--lop-in",
                "--silent",
                "--cpu-limit=30",
                "--soft-cpu-limit=20",
                "--memory-limit=0",
            ),
            "p(a).\n",
        ),
        (
            "invalid-hard-after-soft",
            ("--soft-cpu-limit=10", "--cpu-limit=10"),
            "p(a).\n",
        ),
        (
            "invalid-soft-after-hard",
            ("--cpu-limit=10", "--soft-cpu-limit=10"),
            "p(a).\n",
        ),
        (
            "missing-input",
            ("--lop-in", "missing-eground-input.lop"),
            None,
            {"isolated_workdir": True},
        ),
        (
            "missing-output-parent",
            ("--lop-in", "-o", "missing/ground.out"),
            "p(a).\n",
            {
                "isolated_workdir": True,
                "output_absent_files": ("missing/ground.out",),
            },
        ),
    ),
    "enormalizer": (
        (
            "term-basic",
            ("-t", "{fixture:terms.lop}", "{fixture:rules.lop}"),
            None,
            {
                "rules.lop": "f(X)=a.\n",
                "terms.lop": "f(b)\n",
            },
        ),
        (
            "clause-basic",
            ("--lop-in", "-c", "clauses.lop", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.lop": "f(X)=a.\n",
                    "clauses.lop": "p(f(b)).\n",
                },
            },
        ),
        (
            "tstp-formula-target",
            ("--tstp-in", "--tstp-out", "-f", "formulas.p", "rules.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.p": "cnf(rule,axiom,f(X)=a).\n",
                    "formulas.p": (
                        "fof(with_info,axiom,p(f(b)),"
                        "file('formulas.p',with_info),[status(thm)]).\n"
                    ),
                },
            },
        ),
        (
            "old-tptp-formula-roles",
            ("--tptp-in", "--tptp-out", "-f", "formulas.p", "rules.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.p": "input_clause(rule,axiom,[++equal(f(X),a)]).\n",
                    "formulas.p": (
                        "input_formula(old_axiom,axiom,p(f(a))).\n"
                        "input_formula(17,hypothesis,q(f(b))).\n"
                        'input_formula("old negated",negated_conjecture,r(f(c))).\n'
                        "input_formula(old_conjecture,conjecture,s(f(d))).\n"
                        "input_formula(old_question,question,t(f(e))).\n"
                        "input_formula(lemma_form,lemma,p(f(b))).\n"
                        "input_formula(12,unknown,q(f(c))).\n"
                    ),
                },
            },
        ),
        (
            "tstp-fo-wrapper-matrix",
            ("--tstp-in", "--tstp-out", "-f", "formulas.p", "rules.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.p": "",
                    "formulas.p": (
                        "fof(fof_type,type,fof_symbol:$i,"
                        "file('types.p',fof_type),[status(thm)]).\n"
                        "tff(tff_type,type,tff_symbol:$i).\n"
                        "tcf(tcf_type,type,tcf_symbol:$i).\n"
                        "fof(fof_axiom,axiom,$true,42).\n"
                        "tff(18,hypothesis,$true,[source(foo)]).\n"
                        "fof('quoted-definition',definition,$true,"
                        "file('matrix.p',quoted_definition),[status(thm)]).\n"
                        'fof("string assumption",assumption,$true).\n'
                        "fof(fof_lemma,lemma,$true).\n"
                        "fof(fof_theorem,theorem,$true).\n"
                        "fof(fof_conjecture,conjecture,$true).\n"
                        "fof(fof_question,question,$true).\n"
                        "fof(fof_negated,negated_conjecture,$true).\n"
                        "fof(fof_plain,plain,$true).\n"
                        "fof(fof_unknown,unknown,$true).\n"
                        "tcf(tcf_watch,watchlist,$true).\n"
                    ),
                },
            },
        ),
        (
            "thf-wrapper-matrix",
            ("--tstp-in", "--tstp-out", "-f", "formulas.p", "rules.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.p": "",
                    "formulas.p": (
                        "thf(person_type,type,person:$tType).\n"
                        "thf('quoted-thf',definition,$true,"
                        "file('matrix.p',quoted_thf),[status(thm)]).\n"
                        "thf(19,question,$true).\n"
                    ),
                },
                "reference_mode": "ho",
            },
        ),
        (
            "stdin-include-rules",
            ("--tstp-in", "-t", "terms.p"),
            "include('inc.p').\n",
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "inc.p": "cnf(rule,axiom,f(X)=a).\n",
                    "terms.p": "f(b)\n",
                },
            },
        ),
        (
            "shared-stdin-consumed-by-rules",
            ("--lop-in", "-t", "-"),
            "f(X)=a.\n",
        ),
        (
            "print-statistics-noop",
            ("--lop-in", "--print-statistics", "-t", "terms.lop", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.lop": "f(X)=a.\n",
                    "terms.lop": "f(b)\n",
                },
            },
        ),
        (
            "output-file",
            ("--lop-in", "-t", "terms.lop", "-o", "normalized.out", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.lop": "f(X)=a.\n",
                    "terms.lop": "f(b)\n",
                },
                "output_files": ("normalized.out",),
            },
        ),
        (
            "lop-formula-output-fallback",
            ("--tstp-in", "-f", "formulas.p", "rules.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.p": "",
                    "formulas.p": "fof(form1,axiom,p(a)).\n",
                },
            },
        ),
        (
            "lop-formula-unsupported",
            ("--lop-in", "-f", "formulas.lop", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.lop": "",
                    "formulas.lop": "p(a).\n",
                },
            },
        ),
        (
            "malformed-rule",
            ("--lop-in",),
            "f(a\n",
        ),
        (
            "malformed-term-target",
            ("--lop-in", "-t", "terms.lop", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.lop": "f(X)=a.\n",
                    "terms.lop": "f(b\n",
                },
            },
        ),
        (
            "malformed-clause-target",
            ("--lop-in", "-c", "clauses.lop", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.lop": "f(X)=a.\n",
                    "clauses.lop": "p(f(b).\n",
                },
            },
        ),
        (
            "malformed-formula-target",
            ("--tstp-in", "-f", "formulas.p", "rules.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.p": "cnf(rule,axiom,f(X)=a).\n",
                    "formulas.p": "fof(bad,axiom,p(f(b)).\n",
                },
            },
        ),
        (
            "resource-options-success",
            (
                "--lop-in",
                "--cpu-limit=30",
                "--soft-cpu-limit=20",
                "--memory-limit=0",
                "-t",
                "terms.lop",
                "rules.lop",
            ),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "rules.lop": "f(X)=a.\n",
                    "terms.lop": "f(b)\n",
                },
            },
        ),
        (
            "invalid-hard-after-soft",
            ("--soft-cpu-limit=10", "--cpu-limit=10"),
            None,
        ),
        (
            "invalid-soft-after-hard",
            ("--cpu-limit=10", "--soft-cpu-limit=10"),
            None,
        ),
        (
            "missing-rule",
            ("missing-enormalizer-rules.lop",),
            None,
            {"isolated_workdir": True},
        ),
        (
            "missing-term-target",
            ("--lop-in", "-t", "missing-terms.lop", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {"rules.lop": "f(X)=a.\n"},
            },
        ),
        (
            "missing-clause-target",
            ("--lop-in", "-c", "missing-clauses.lop", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {"rules.lop": "f(X)=a.\n"},
            },
        ),
        (
            "missing-formula-target",
            ("--tstp-in", "-f", "missing-formulas.p", "rules.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {"rules.p": "cnf(rule,axiom,f(X)=a).\n"},
            },
        ),
        (
            "missing-output-parent",
            ("--lop-in", "-o", "missing/normalized.out", "rules.lop"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {"rules.lop": "f(X)=a.\n"},
                "output_absent_files": ("missing/normalized.out",),
            },
        ),
    ),
    "epclanalyse": (
        (
            "stdin-basic",
            (),
            (
                "1 : : [++p(a)] : initial\n"
                "2 : : [++q(a),--r(X)] : 1 : 'derived'\n"
                "3 : : [] : 2\n"
            ),
        ),
        (
            "zero-denominator-safe-boundary",
            (),
            "1 : : p(a) : initial\n2 : : [] : 1\n",
        ),
        (
            "missing-input",
            ("missing-epclanalyse-input.pcl",),
            None,
            {"isolated_workdir": True},
        ),
    ),
    "epclextract": (
        (
            "stdin-basic",
            (),
            (
                "1 : : [++p] : initial\n"
                "2 : lemma : [++q] : 1\n"
                "3 : : [] : 2 : 'final'\n"
                "4 : : [++r] : initial\n"
            ),
        ),
        (
            "mixed-logic-proof-closure",
            (),
            (
                "1 : : p(a) : initial\n"
                "2 : : : 1\n"
                "3 : : q(a)|r(b) : 2\n"
                "4 : lemma : [++s(a)] : pm(1,3)\n"
                "5 : : : 4 : 'final'\n"
                "6 : : [++unused] : initial\n"
            ),
        ),
        (
            "multi-file-comments",
            (
                "--forward-comments",
                "{fixture:first.pcl}",
                "{fixture:second.pcl}",
            ),
            None,
            {
                "first.pcl": (
                    "% first lead\n"
                    "1 : : p(a) : initial\n"
                    "% first tail\n"
                ),
                "second.pcl": (
                    "% second lead\n"
                    "2 : : : 1 : 'final'\n"
                    "% second tail\n"
                ),
            },
        ),
        (
            "missing-input",
            ("missing-epclextract-input.pcl",),
            None,
            {"isolated_workdir": True},
        ),
    ),
    "epcllemma": (
        (
            "stdin-basic",
            ("--max-lemmas=0", "--min-lemma-quality=0"),
            (
                "1 : : [++p(a)] : initial\n"
                "2 : : [++q(a)] : initial\n"
                "3 : : [++r(a)] : pm(1,2)\n"
                "4 : : [++s(a)] : pm(1,3)\n"
                "5 : : [++t(a)] : er(4)\n"
            ),
        ),
        (
            "large-relative-limit",
            ("--min-lemma-quality=0",),
            EPCLLEMMA_LARGE_PROTOCOL,
        ),
        (
            "formula-lemma-pcl",
            ("--max-lemmas=0", "--min-lemma-quality=0"),
            EPCLLEMMA_FORMULA_PROTOCOL,
        ),
        (
            "formula-lemma-tptp",
            ("--max-lemmas=0", "--min-lemma-quality=0", "--tptp-out"),
            EPCLLEMMA_FORMULA_PROTOCOL,
        ),
        (
            "formula-lemma-tstp",
            ("--max-lemmas=0", "--min-lemma-quality=0", "--tstp-out"),
            EPCLLEMMA_FORMULA_PROTOCOL,
        ),
        (
            "formula-lemma-lop",
            ("--max-lemmas=0", "--min-lemma-quality=0", "--lop-out"),
            EPCLLEMMA_FORMULA_PROTOCOL,
        ),
        ("minimum-quality-nan", ("--min-lemma-quality=nan",), ""),
        ("minimum-quality-positive-infinity", ("--min-lemma-quality=inf",), ""),
        ("minimum-quality-negative-infinity", ("--min-lemma-quality=-inf",), ""),
        ("minimum-quality-negative-zero", ("--min-lemma-quality=-0",), ""),
        ("shell-step-rejection", (), "1 : : : initial\n"),
        (
            "missing-input",
            ("missing-epcllemma-input.pcl",),
            None,
            {"isolated_workdir": True},
        ),
        (
            "missing-output-parent",
            ("--output-file=missing-parent/lemmas.pcl",),
            "",
            {"isolated_workdir": True},
        ),
    ),
    "epatternize": (
        ("lop-basic", ("--lop-in",), "p(a).\n"),
        ("lop-unrecognized-tail", ("--lop-in",), "p(a). ) q(a).\n"),
        (
            "tptp-unrecognized-tail",
            ("--tptp-in",),
            "input_formula(f,axiom,p(a)). bogus_record(x).\n",
        ),
        (
            "tstp-unrecognized-tail",
            ("--tstp-in",),
            "fof(f,axiom,p(a)). bogus_record(x).\n",
        ),
        (
            "old-tptp-record-mix",
            ("--tptp-in",),
            (
                "input_formula(old_formula,axiom,p(a)).\n"
                "input_clause(old_clause,axiom,[++q(b),--r(X)]).\n"
            ),
        ),
        (
            "tstp-mixed-corpus",
            ("--tstp-in",),
            (
                "tff(a_type,type,a:$i).\n"
                "tff(b_type,type,b:$i).\n"
                "tff(f_type,type,f:$i>$i).\n"
                "tff(p_type,type,p:$i>$o).\n"
                "tff(q_type,type,q:$i>$o).\n"
                "fof(unit_formula,axiom,p(f(a))).\n"
                "fof(conjunction,axiom,(p(a)&q(b))).\n"
                "fof(implication,axiom,(p(a)=>q(a))).\n"
                "fof(existential,axiom,?[X]:(p(X)&q(f(X)))).\n"
                "tff(typed_formula,axiom,![X:$i]:(p(X)|q(f(X)))).\n"
                "tcf(typed_clause,axiom,![X:$i]:(p(X)|~q(X))).\n"
                "cnf(binary_clause,axiom,(p(a)|~q(b))).\n"
                "cnf(equality_clause,axiom,(f(a)=b|p(b))).\n"
                "cnf(watch,watchlist,p(a)).\n"
            ),
        ),
        (
            "nested-selected-include",
            ("--tstp-in", "main.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "main.p": (
                        "include('child.p',[old_formula,old_clause,modern_formula,"
                        "modern_clause,typed_formula,tcf_formula,watch_clause]).\n"
                        "fof(local_formula,axiom,local(a)).\n"
                    ),
                    "child.p": (
                        "include('grandchild.p',[old_formula,old_clause,"
                        "modern_formula,modern_clause]).\n"
                        "tff(a_type,type,a:$i).\n"
                        "tff(typed_pred_type,type,typed_pred:$i>$o).\n"
                        "tff(typed_formula,axiom,typed_pred(a)).\n"
                        "tcf(tcf_formula,axiom,![X:$i]:typed_pred(X)).\n"
                        "tcf(watch_clause,watchlist,typed_pred(a)).\n"
                        "fof(dropped_child,axiom,dropped(a)).\n"
                    ),
                    "grandchild.p": (
                        "fof(old_formula,axiom,oldp(a)).\n"
                        "cnf(old_clause,axiom,oldq(a)).\n"
                        "fof(modern_formula,axiom,modernp(a),"
                        "file('grandchild.p',modern_formula),[status(thm)]).\n"
                        "cnf(modern_clause,axiom,modernq(a),"
                        "file('grandchild.p',modern_clause),[status(thm)]).\n"
                        "fof(dropped_grandchild,axiom,nope(a)).\n"
                    ),
                },
            },
        ),
        (
            "multi-file-output",
            ("--tstp-in", "-o", "patterns.out", "first.p", "second.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {
                    "first.p": "fof(first,axiom,p(a)).\n",
                    "second.p": "cnf(second,axiom,q(b)).\n",
                },
                "output_files": ("patterns.out",),
                "expected_mismatches": (
                    "exit_code",
                    "shape",
                    "normalized_stderr",
                    "output_files",
                ),
            },
        ),
        ("malformed-lop", ("--lop-in",), "p(f(a).\n"),
        ("malformed-tstp", ("--tstp-in",), "fof(bad,axiom,p(a).\n"),
        (
            "missing-include",
            ("--tstp-in", "main.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {"main.p": "include('missing-include.p').\n"},
            },
        ),
        (
            "missing-input",
            ("missing-epatternize-input.p",),
            None,
            {"isolated_workdir": True},
        ),
        (
            "missing-output-parent",
            ("--tstp-in", "-o", "missing/patterns.out", "problem.p"),
            None,
            {
                "isolated_workdir": True,
                "workdir_files": {"problem.p": "fof(a,axiom,p(a)).\n"},
                "output_absent_files": ("missing/patterns.out",),
            },
        ),
        ("invalid-class-mask", ("--class-mask=short",), None),
    ),
    "ex_commandline": (
        ("options-basic", ("--int_example=42", "--float_example", "one.p", "two.p"), None),
        ("unknown-long-option", ("--unknown",), None),
        ("missing-required-argument", ("--int_example",), None),
        ("invalid-integer", ("--int_example=bad",), None),
        ("integer-range", ("--int_example=9223372036854775808",), None),
        ("float-range", ("--float_example=1e9999",), None),
    ),
    "term2dag": (
        ("stdin-basic", (), "f(a,a) g(f(a,a))\n"),
        (
            "shared-typed-boundary",
            (),
            (
                "a f(a,a) g(f(a,a)) "
                "h(g(f(a,a)),f(a,a),X:$i) h(g(f(a,a)),f(a,a),X) "
                "apply(F:$i > $i,a) apply(F,a) q(Y:$o) q(Y) 42 \"obj\"\n"
            ),
            {
                "reference_mode": "ho",
                "expected_mismatches": ("normalized_stdout",),
            },
        ),
        (
            "missing-input",
            ("missing-term2dag-input",),
            None,
            {"isolated_workdir": True},
        ),
    ),
    "termprops": (
        ("stdin-basic", (), "a f(a,a) g(f(a),a)\n"),
        ("empty-input", (), ""),
        (
            "missing-input",
            ("missing-termprops-input",),
            None,
            {"isolated_workdir": True},
        ),
    ),
    "tsm_classify": (
        (
            "stdin-basic",
            ("--index-type=IndexIdentity", "--tsm-type=Flat"),
            (
                "Training:\n"
                "a : 1:(1,-1).\n"
                "f(a) : 2:(1,1).\n"
                ".\n"
                "Test:\n"
                "a : 1:(1,-1).\n"
                "f(a) : 2:(1,1).\n"
                ".\n"
            ),
        ),
        (
            "recursive-mixed",
            (
                "--index-type=IndexSymbol",
                "--index-depth=3",
                "--tsm-type=Recursive",
            ),
            TSM_RECURSIVE_CORPUS,
        ),
        (
            "empty-test-set",
            ("--index-type=IndexIdentity", "--tsm-type=Flat"),
            "Training:\na : 1:(1,-1).\nf(a) : 2:(1,1).\n.\nTest:\n.\n",
        ),
    ),
}


class InteropError(RuntimeError):
    pass


def run_checked(
    command: Sequence[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    capture: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        list(map(str, command)),
        cwd=cwd,
        env=env,
        text=True,
        errors="replace",
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout or "").strip()
        raise InteropError(
            f"Command failed ({result.returncode}): {' '.join(map(str, command))}"
            + (f"\n{detail}" if detail else "")
        )
    return result


def cache_root() -> Path:
    configured = os.environ.get("E_RUST_PORT_COMPAT_ROOT")
    if configured:
        return Path(configured)
    return Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "e-rust-port"


def artifact_root() -> Path:
    configured = os.environ.get("E_RUST_PORT_COMPAT_ARTIFACT_ROOT")
    if configured:
        return Path(configured)
    return Path.cwd() / ".artifacts" / "e-compare"


def reference_manifest_path() -> Path:
    return cache_root() / "reference.json"


def safe_remove_cache_path(path: Path) -> None:
    cache = cache_root().resolve()
    resolved = path.resolve()
    if resolved == cache or cache not in resolved.parents:
        raise InteropError(f"Refusing to remove path outside the managed cache: {resolved}")
    shutil.rmtree(resolved)


def os_release() -> dict[str, str]:
    values: dict[str, str] = {}
    release = Path("/etc/os-release")
    if release.exists():
        for line in release.read_text(encoding="utf-8").splitlines():
            if "=" in line:
                key, value = line.split("=", 1)
                values[key] = value.strip().strip('"')
    return values


def first_line(command: Sequence[str], *, env: dict[str, str] | None = None) -> str:
    return run_checked(command, env=env).stdout.splitlines()[0].strip()


def environment_with_path_prefix(*directories: Path) -> dict[str, str]:
    environment = os.environ.copy()
    prefixes = [str(directory) for directory in directories if directory.is_dir()]
    if prefixes:
        existing = environment.get("PATH", "")
        environment["PATH"] = (
            os.pathsep.join([*prefixes, existing])
            if existing
            else os.pathsep.join(prefixes)
        )
    return environment


def rust_tool_environment() -> dict[str, str]:
    return environment_with_path_prefix(Path.home() / ".cargo" / "bin")


def prepare_reference_source(source: Path, destination: Path) -> None:
    """Copy the uploaded C source and normalize only the disposable build tree."""

    shutil.copytree(
        source,
        destination,
        ignore=shutil.ignore_patterns(".git", ".dolt", "__pycache__"),
    )
    for candidate in destination.rglob("*"):
        if not candidate.is_file():
            continue
        try:
            data = candidate.read_bytes()
        except OSError:
            continue
        if b"\0" not in data and b"\r\n" in data:
            candidate.write_bytes(data.replace(b"\r\n", b"\n"))
            data = data.replace(b"\r\n", b"\n")
        if data.startswith(b"#!"):
            candidate.chmod(candidate.stat().st_mode | 0o100)


def build_one(source: Path, commit: str, mode: str) -> dict[str, Any]:
    build_dir = cache_root() / "sources" / commit / mode
    binary_name = "eprover-ho" if mode == "ho" else "eprover"
    installed_binary = cache_root() / "bin" / commit / mode / binary_name

    if build_dir.exists():
        safe_remove_cache_path(build_dir)
    build_dir.parent.mkdir(parents=True, exist_ok=True)
    prepare_reference_source(source, build_dir)

    configure = ["./configure"] + (["--enable-ho"] if mode == "ho" else [])
    run_checked(configure, cwd=build_dir, capture=False)
    (build_dir / "PROVER" / "e_gitcommit.h").write_text(
        f'#define ECOMMITID "{commit}"\n',
        encoding="utf-8",
    )
    jobs = str(max(1, os.cpu_count() or 1))
    run_checked(["make", "-j", jobs], cwd=build_dir, capture=False)

    built_binary = build_dir / "PROVER" / binary_name
    if not built_binary.is_file():
        raise InteropError(f"Expected reference binary was not built: {built_binary}")
    installed_binary.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(built_binary, installed_binary)
    installed_binary.chmod(installed_binary.stat().st_mode | 0o111)
    tools = copy_reference_tools(build_dir, commit) if mode == "fol" else {}

    version = run_checked([str(installed_binary), "--version"]).stdout.strip()
    help_result = subprocess.run(
        [str(installed_binary), "--help"],
        text=True,
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if help_result.returncode != 0 or not help_result.stdout.strip():
        raise InteropError(f"{binary_name} --help did not complete successfully")

    smoke_problem = (
        build_dir / "EXAMPLE_PROBLEMS" / "LFHOL" / "permute_func_no_axioms.p"
        if mode == "ho"
        else build_dir / "EXAMPLE_PROBLEMS" / "SMOKETEST" / "socrates.p"
    )
    smoke_env = os.environ.copy()
    smoke_env["TPTP"] = str(smoke_problem.parent)
    smoke = run_checked(
        [
            str(installed_binary),
            str(smoke_problem),
            "--auto",
            "--silent",
            "--cpu-limit=10",
        ],
        cwd=smoke_problem.parent,
        env=smoke_env,
    )
    smoke_status = szs_status(smoke.stdout)
    if smoke_status is None:
        raise InteropError(f"{binary_name} smoke test did not emit an SZS status")

    return {
        "mode": mode,
        "configure": configure[1:],
        "binary": str(installed_binary),
        "build_source": str(build_dir),
        "version": version,
        "smoke_status": smoke_status,
        "sha256": sha256_file(installed_binary),
        "tools": tools,
    }


def copy_reference_tools(build_dir: Path, commit: str) -> dict[str, str]:
    tools: dict[str, str] = {}
    for name, relative in sorted(REFERENCE_TOOL_BINARIES.items()):
        built_binary = build_dir / relative
        if not built_binary.is_file() and name in ARCHIVED_REFERENCE_TOOL_LINKS:
            build_archived_reference_tool(build_dir, name)
        if not built_binary.is_file():
            raise InteropError(f"Expected reference tool was not built: {built_binary}")
        installed_binary = cache_root() / "bin" / commit / "tools" / name
        installed_binary.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(built_binary, installed_binary)
        installed_binary.chmod(installed_binary.stat().st_mode | 0o111)
        tools[name] = str(installed_binary)
    return tools


def build_archived_reference_tool(build_dir: Path, name: str) -> None:
    try:
        compile_command, link_command = ARCHIVED_REFERENCE_TOOL_LINKS[name]
    except KeyError as error:
        raise InteropError(f"No archived reference-tool build is configured for {name}") from error
    apply_archived_reference_tool_source_patches(build_dir, name)
    run_checked(compile_command, cwd=build_dir / "PROVER", capture=False)
    run_checked(link_command, cwd=build_dir / "PROVER", capture=False)


def apply_archived_reference_tool_source_patches(build_dir: Path, name: str) -> None:
    for relative, old, new in ARCHIVED_REFERENCE_TOOL_SOURCE_PATCHES.get(name, ()):
        source = build_dir / relative
        text = source.read_text(encoding="utf-8")
        if new in text:
            continue
        if old not in text:
            raise InteropError(
                f"Could not apply archived reference-tool compatibility patch for "
                f"{name}: {relative}"
            )
        source.write_text(text.replace(old, new, 1), encoding="utf-8")


def build_reference(args: argparse.Namespace) -> None:
    repo_root = args.repo_root.resolve()
    source = repo_root / "eprover"
    if not (source / "configure").is_file():
        raise InteropError(f"Expected the uploaded upstream source at {source}")
    commit = args.eprover_commit
    if re.fullmatch(r"[A-Za-z0-9._-]+", commit) is None:
        raise InteropError(f"Invalid upstream commit identifier: {commit}")

    print(f"Building E reference commit {commit} in {cache_root()}", flush=True)
    builds = [build_one(source, commit, mode) for mode in ("fol", "ho")]

    release = os_release()
    manifest = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "upstream_commit": commit,
        "upstream_source": str(source),
        "compiler": first_line(["gcc", "--version"]),
        "make": first_line(["make", "--version"]),
        "platform": platform.platform(),
        "distribution": {
            "id": release.get("ID", "unknown"),
            "version": release.get("VERSION_ID", "unknown"),
            "pretty_name": release.get("PRETTY_NAME", "unknown"),
        },
        "builds": {build["mode"]: build for build in builds},
    }
    manifest_path = reference_manifest_path()
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    write_json(manifest_path, manifest)
    print(f"Reference manifest: {manifest_path}")
    for build in builds:
        print(f"  {build['mode']}: {build['binary']}")


def load_manifest() -> dict[str, Any]:
    path = reference_manifest_path()
    if not path.is_file():
        raise InteropError(
            f"Reference manifest not found at {path}. Run build-reference first."
        )
    manifest = json.loads(path.read_text(encoding="utf-8"))
    for mode in ("fol", "ho"):
        binary = Path(manifest["builds"][mode]["binary"])
        if not binary.is_file():
            raise InteropError(f"Reference binary is missing: {binary}")
    for tool, binary_name in manifest["builds"]["fol"].get("tools", {}).items():
        binary = Path(binary_name)
        if not binary.is_file():
            raise InteropError(f"Reference tool {tool} is missing: {binary}")
    return manifest


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def cross_platform_path_replacements(
    path: Path, placeholder: str
) -> list[tuple[str, str]]:
    forms = (str(path), str(path.resolve()))
    unique_forms = (form for form in dict.fromkeys(forms) if form)
    return [
        (form, placeholder)
        for form in sorted(unique_forms, key=len, reverse=True)
    ]


def tool_binary_path_replacements(
    reference_binary: Path,
    candidate_binary: Path,
    tool: str,
) -> list[tuple[str, str]]:
    return [
        *cross_platform_path_replacements(reference_binary, tool),
        *cross_platform_path_replacements(candidate_binary, tool),
    ]


def expected_status(text: str) -> str | None:
    match = EXPECTED_RE.search(text)
    return match.group(1) if match else None


def szs_status(text: str) -> str | None:
    matches = SZS_RE.findall(text)
    return matches[-1] if matches else None


def normalize_output(
    text: str,
    replacements: Iterable[tuple[str, str]] = (),
    *,
    normalize_legacy_classify_feature_suffix: bool = False,
) -> str:
    normalized = text.replace("\r\n", "\n")
    for old, new in replacements:
        if old:
            normalized = normalized.replace(old, new)
    lines = [
        normalize_platform_line(line.rstrip())
        for line in normalized.splitlines()
        if not VOLATILE_LINE_RE.search(line)
    ]
    if normalize_legacy_classify_feature_suffix:
        lines = [normalize_classify_legacy_feature_suffix(line) for line in lines]
    lines = normalize_app_encode_type_declarations(lines)
    lines = normalize_saturation_blocks(lines)
    return "\n".join(lines).strip()


def normalize_platform_line(line: str) -> str:
    normalized = PLATFORM_NAN_PERCENT_RE.sub("successes, <NAN> percent", line)
    if normalized.startswith("% Terms: "):
        normalized = PLATFORM_TERMPROPS_NAN_RE.sub(r"\g<label><NAN>", normalized)
    if normalized.startswith(EPCLANALYSE_AVERAGE_PREFIXES):
        normalized = PLATFORM_EPCLANALYSE_NAN_RE.sub(r"\g<label> <NAN>", normalized)
    if normalized.startswith(("% Running ", "%> ")):
        normalized = PLATFORM_PROOFCHECK_TEMP_RE.sub("<PROOFCHECK_TMP>", normalized)
    if LEGACY_SERVER_ACCEPTED_DESCRIPTOR_RE.fullmatch(normalized):
        return "Accepted <DESCRIPTOR>"
    for replacement, suffixes in PLATFORM_ERROR_SUFFIXES.items():
        for suffix in suffixes:
            if normalized.endswith(suffix):
                return normalized[: -len(suffix)] + replacement
    return normalized


def normalize_classify_legacy_feature_suffix(line: str) -> str:
    """Canonicalize only the fields that C leaves uninitialized after legacy parsing."""

    match = CLASSIFY_LEGACY_FEATURE_SUFFIX_RE.fullmatch(line)
    if match is None:
        return line
    return (
        f"{match.group('prefix')}, <UNINITIALIZED-LEGACY-SUFFIX> ) : "
        f"{match.group('class_prefix')}<UNINITIALIZED-LEGACY-CLASS>"
    )


def normalize_app_encode_type_declarations(lines: Iterable[str]) -> list[str]:
    """Sort and renumber C app-encoded type declarations by stable type UID."""

    source = list(lines)
    result: list[str] = []
    index = 0
    while index < len(source):
        entries: list[tuple[int, str | None, str]] = []
        while index < len(source):
            comment: str | None = None
            declaration_index = index
            if (
                source[index].startswith("%-- ")
                and index + 1 < len(source)
                and APP_ENCODE_TYPE_DECL_RE.match(source[index + 1])
            ):
                comment = source[index]
                declaration_index += 1

            declaration = source[declaration_index]
            match = APP_ENCODE_TYPE_DECL_RE.match(declaration)
            if not match:
                break
            entries.append((int(match.group(1)), comment, declaration))
            index = declaration_index + 1

        if entries:
            for ordinal, (_, comment, declaration) in enumerate(
                sorted(entries, key=lambda entry: entry[0]), 1
            ):
                if comment is not None:
                    result.append(comment)
                result.append(re.sub(r"typedecl\d+", f"typedecl{ordinal}", declaration, count=1))
            continue

        result.append(source[index])
        index += 1

    return result


def normalize_saturation_blocks(lines: Iterable[str]) -> list[str]:
    """Sort saturation listings while preserving proof order.

    E can emit the same saturated clause/formula set in a different order across
    two runs of the same binary.  That is not a semantic proof-output mismatch.
    Actual refutation/proof blocks remain order-sensitive because proof order is
    part of the derivation structure.
    """

    result: list[str] = []
    saturation_body: list[str] | None = None

    def flush_saturation_body() -> None:
        nonlocal saturation_body
        if saturation_body is not None:
            result.extend(sorted(saturation_body, key=lambda line: line.strip()))
            saturation_body = None

    for line in lines:
        start = SZS_OUTPUT_START_RE.search(line)
        if start and start.group(1).lower() == "saturation":
            flush_saturation_body()
            result.append(line)
            saturation_body = []
            continue

        if saturation_body is not None:
            end = SZS_OUTPUT_END_RE.search(line)
            if end:
                flush_saturation_body()
                result.append(line)
            else:
                saturation_body.append(normalize_saturation_line(line))
            continue

        result.append(line)

    flush_saturation_body()
    return result


def normalize_saturation_line(line: str) -> str:
    return SATURATION_GENERATED_ID_RE.sub("<CLAUSE_ID>", line)


def output_shape(stdout: str, stderr: str) -> dict[str, Any]:
    return {
        "szs_status_count": len(SZS_RE.findall(stdout)),
        "proof_start_count": len(re.findall(r"SZS output start", stdout, re.IGNORECASE)),
        "proof_end_count": len(re.findall(r"SZS output end", stdout, re.IGNORECASE)),
        "stdout_nonempty": bool(stdout.strip()),
        "stderr_nonempty": bool(stderr.strip()),
    }


def execute(
    executable: Path,
    arguments: Sequence[str],
    *,
    timeout: int,
    env: dict[str, str],
    stdin_text: str | None = None,
    cwd: Path | None = None,
) -> dict[str, Any]:
    command = [str(executable), *map(str, arguments)]
    started = time.perf_counter()
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            input=stdin_text,
            text=True,
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        timed_out = False
        exit_code: int | None = result.returncode
        stdout = result.stdout
        stderr = result.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        exit_code = None
        stdout = error.stdout or ""
        stderr = error.stderr or ""
        if isinstance(stdout, bytes):
            stdout = stdout.decode("utf-8", "replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode("utf-8", "replace")
    elapsed = time.perf_counter() - started
    return {
        "command": command,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "wall_seconds": elapsed,
        "stdout": stdout,
        "stderr": stderr,
        "status": szs_status(stdout),
        "shape": output_shape(stdout, stderr),
    }


def enumerate_problems(repo_root: Path, corpus: Path | None) -> list[dict[str, Any]]:
    if corpus:
        roots = [(corpus, "ho" if "lfhol" in str(corpus).lower() else "fol")]
    else:
        examples = repo_root / "eprover" / "EXAMPLE_PROBLEMS"
        roots = [
            (examples / "SMOKETEST", "fol"),
            (examples / "TPTP", "fol"),
            (examples / "LFHOL", "ho"),
        ]

    cases: list[dict[str, Any]] = []
    seen: set[Path] = set()
    for root, mode in roots:
        if not root.is_dir():
            raise InteropError(f"Corpus directory does not exist: {root}")
        for problem in sorted(root.rglob("*"), key=lambda item: str(item).lower()):
            if not problem.is_file() or problem.suffix.lower() not in PROBLEM_SUFFIXES:
                continue
            if "axioms" in {part.lower() for part in problem.parts}:
                continue
            resolved = problem.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            text = problem.read_text(encoding="utf-8", errors="replace")
            problem_mode = (
                "ho"
                if mode == "ho"
                or "^" in problem.name
                or re.search(r"^\s*thf\s*\(", text, re.MULTILINE | re.IGNORECASE)
                else "fol"
            )
            cases.append(
                {
                    "name": str(problem.relative_to(root)),
                    "path": problem,
                    "mode": problem_mode,
                    "expected_status": expected_status(text),
                    "stdin": None,
                    "scenario": "file",
                }
            )
    if not cases:
        raise InteropError("The selected corpus does not contain any .p or .lop problems")
    return cases


def tptp_root(repo_root: Path, corpus: Path | None, problem: Path) -> Path:
    if corpus:
        return corpus
    bundled = repo_root / "eprover" / "EXAMPLE_PROBLEMS" / "TPTP"
    return bundled if "TPTP" in problem.parts else problem.parent


def common_arguments(timeout: int, memory_limit_mb: int, proof: bool) -> list[str]:
    arguments = [
        "--auto",
        "--silent",
        f"--cpu-limit={timeout}",
        f"--memory-limit={memory_limit_mb}",
        "--detsort-rw",
        "--detsort-new",
    ]
    if proof:
        arguments.append("--proof-object=1")
    return arguments


def comparison_cpu_limit(case: dict[str, Any], default: int) -> int:
    """Return an exact case override or the case/default minimum, in that order."""

    if "cpu_limit" in case:
        return int(case["cpu_limit"])
    return max(default, int(case.get("minimum_cpu_limit", 0)))


def comparison_cases(
    repo_root: Path,
    corpus: Path | None,
    run_dir: Path,
) -> list[dict[str, Any]]:
    cases = enumerate_problems(repo_root, corpus)
    if corpus:
        return cases

    socrates = repo_root / "eprover" / "EXAMPLE_PROBLEMS" / "SMOKETEST" / "socrates.p"
    if socrates.is_file():
        text = socrates.read_text(encoding="utf-8", errors="replace")
        cases.append(
            {
                "name": "synthetic/stdin-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": expected_status(text),
                "stdin": text,
                "scenario": "stdin",
            }
        )
        cases.append(
            {
                "name": "synthetic/syntax-only-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": "Unknown",
                "stdin": None,
                "scenario": "syntax-only",
                "arguments": ("--syntax-only",),
            }
        )
        cases.append(
            {
                "name": "synthetic/stdin-syntax-only-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": "Unknown",
                "stdin": text,
                "scenario": "stdin-syntax-only",
                "arguments": ("--syntax-only",),
            }
        )
        cases.append(
            {
                "name": "synthetic/print-formulas-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": None,
                "stdin": None,
                "scenario": "print-formulas",
                "arguments": ("--print-formulas",),
            }
        )
        cases.append(
            {
                "name": "synthetic/stdin-print-formulas-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": None,
                "stdin": text,
                "scenario": "stdin-print-formulas",
                "arguments": ("--print-formulas",),
            }
        )
        cases.append(
            {
                "name": "synthetic/prune-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": "Unknown",
                "stdin": None,
                "scenario": "prune",
                "arguments": ("--prune",),
            }
        )
        cases.append(
            {
                "name": "synthetic/stdin-prune-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": "Unknown",
                "stdin": text,
                "scenario": "stdin-prune",
                "arguments": ("--prune",),
            }
        )
        cases.append(
            {
                "name": "synthetic/cnf-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": "Unknown",
                "stdin": None,
                "scenario": "cnf",
                "arguments": ("--cnf",),
            }
        )
        cases.append(
            {
                "name": "synthetic/stdin-cnf-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": "Unknown",
                "stdin": text,
                "scenario": "stdin-cnf",
                "arguments": ("--cnf",),
            }
        )
        cases.append(
            {
                "name": "synthetic/app-encode-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": None,
                "stdin": None,
                "scenario": "app-encode",
                "arguments": ("--app-encode",),
            }
        )
        cases.append(
            {
                "name": "synthetic/stdin-app-encode-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": None,
                "stdin": text,
                "scenario": "stdin-app-encode",
                "arguments": ("--app-encode",),
            }
        )

    malformed = run_dir / "fixtures" / "malformed.p"
    malformed.parent.mkdir(parents=True, exist_ok=True)
    malformed.write_text("fof(broken, conjecture, (\n", encoding="utf-8")
    cases.append(
        {
            "name": "synthetic/malformed.p",
            "path": malformed,
            "mode": "fol",
            "expected_status": None,
            "stdin": None,
            "scenario": "malformed",
        }
    )

    harder = repo_root / "eprover" / "PROVER" / "LUSK6.lop"
    if harder.is_file():
        cases.append(
            {
                "name": "synthetic/cpu-limit-LUSK6.lop",
                "path": harder,
                "mode": "fol",
                "expected_status": None,
                "stdin": None,
                "scenario": "cpu-limit",
                "cpu_limit": 1,
            }
        )

    if socrates.is_file():
        cases.append(
            {
                "name": "synthetic/memory-limit-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": expected_status(
                    socrates.read_text(encoding="utf-8", errors="replace")
                ),
                "stdin": None,
                "scenario": "memory-limit",
                "memory_limit_mb": 16,
            }
        )
    # These cases intentionally consume their full memory/time allowance. Run
    # them after all functional cases so retained worker memory cannot turn a
    # later proof comparison into a spurious resource failure.
    cases.sort(
        key=lambda case: case["name"] in MAIN_COMPARISON_RESOURCE_STRESS_CASES
    )
    for case in cases:
        minimum_cpu_limit = MAIN_COMPARISON_MINIMUM_CPU_LIMITS.get(case["name"])
        if minimum_cpu_limit is not None:
            case["minimum_cpu_limit"] = minimum_cpu_limit
        case["expected_mismatches"] = list(
            MAIN_COMPARISON_EXPECTED_MISMATCHES.get(
                (case["mode"], case["name"]),
                (),
            )
        )
    return cases


def comparison_mismatches(reference: dict[str, Any], candidate: dict[str, Any]) -> list[str]:
    mismatches: list[str] = []
    for field in ("exit_code", "timed_out", "status", "shape"):
        if reference[field] != candidate[field]:
            mismatches.append(field)
    return mismatches


def mismatch_expectation_matches(
    mismatches: Sequence[str], expected_mismatches: Sequence[str]
) -> bool:
    """Return whether every observed mismatch is an explicitly allowed one."""

    return set(mismatches).issubset(expected_mismatches)


def compare(args: argparse.Namespace) -> None:
    manifest = load_manifest()
    repo_root = args.repo_root.resolve()
    output_root = artifact_root() / "main"
    run_id = dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    run_dir = output_root / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    corpus = args.corpus.resolve() if args.corpus else None
    cases = comparison_cases(repo_root, corpus, run_dir)

    if args.self_test:
        candidate_kind = "linux-reference"
        rust_binary = None
    else:
        rust_binary = args.rust_bin.resolve()
        if not rust_binary.is_file():
            raise InteropError(f"Native Linux Rust executable not found: {rust_binary}")
        candidate_kind = "linux-rust"

    records: list[dict[str, Any]] = []
    mismatch_count = 0
    expected_difference_count = 0
    for index, case in enumerate(cases, 1):
        print(f"[{index}/{len(cases)}] {case['mode']} {case['name']}", flush=True)
        reference_binary = Path(manifest["builds"][case["mode"]]["binary"])
        problem: Path = case["path"]
        tptp = tptp_root(repo_root, corpus, problem)
        fixed_arguments = case.get("arguments")
        case_cpu_limit = comparison_cpu_limit(case, args.timeout)
        if fixed_arguments is None:
            proof = case["scenario"] == "file"
            case_memory_limit = case.get("memory_limit_mb", args.memory_limit_mb)
            options = common_arguments(case_cpu_limit, case_memory_limit, proof)
        else:
            options = list(fixed_arguments)
        reference_args = options if case["stdin"] is not None else [str(problem), *options]
        reference_env = os.environ.copy()
        reference_env["TPTP"] = str(tptp)
        reference = execute(
            reference_binary,
            reference_args,
            timeout=case_cpu_limit + PROCESS_TIMEOUT_GRACE_SECONDS,
            env=reference_env,
            stdin_text=case["stdin"],
            cwd=problem.parent,
        )

        if args.self_test:
            candidate_binary = reference_binary
            candidate_args = reference_args
            candidate_env = reference_env
            candidate_cwd = problem.parent
        else:
            assert rust_binary is not None
            candidate_binary = rust_binary
            candidate_args = (
                options if case["stdin"] is not None else [str(problem), *options]
            )
            candidate_env = os.environ.copy()
            candidate_env["TPTP"] = str(tptp)
            candidate_cwd = problem.parent

        candidate = execute(
            candidate_binary,
            candidate_args,
            timeout=case_cpu_limit + PROCESS_TIMEOUT_GRACE_SECONDS,
            env=candidate_env,
            stdin_text=case["stdin"],
            cwd=candidate_cwd,
        )
        mismatches = comparison_mismatches(reference, candidate)

        replacements = [
            *cross_platform_path_replacements(problem, "<PROBLEM>"),
            *cross_platform_path_replacements(tptp, "<TPTP>"),
        ]
        reference_normalized = normalize_output(reference["stdout"], replacements)
        candidate_normalized = normalize_output(candidate["stdout"], replacements)
        normalized_output_equal = reference_normalized == candidate_normalized
        if not normalized_output_equal and case["scenario"] != "cpu-limit":
            mismatches.append("normalized_stdout")

        expected_mismatches = (
            [] if args.self_test else list(case.get("expected_mismatches", ()))
        )
        mismatch_expectation_met = mismatch_expectation_matches(
            mismatches, expected_mismatches
        )
        if mismatches and mismatch_expectation_met:
            expected_difference_count += 1
        if not mismatch_expectation_met:
            mismatch_count += 1
        if mismatches or not mismatch_expectation_met:
            difference_kind = (
                "expected-differences" if mismatch_expectation_met else "mismatches"
            )
            mismatch_dir = run_dir / difference_kind / f"{index:04d}"
            mismatch_dir.mkdir(parents=True, exist_ok=True)
            for label, result in (("reference", reference), ("candidate", candidate)):
                (mismatch_dir / f"{label}.stdout").write_text(result["stdout"], encoding="utf-8")
                (mismatch_dir / f"{label}.stderr").write_text(result["stderr"], encoding="utf-8")
            (mismatch_dir / "reference.normalized").write_text(reference_normalized, encoding="utf-8")
            (mismatch_dir / "candidate.normalized").write_text(candidate_normalized, encoding="utf-8")

        records.append(
            {
                "name": case["name"],
                "scenario": case["scenario"],
                "mode": case["mode"],
                "expected_status": case["expected_status"],
                "arguments": options,
                "reference_status": reference["status"],
                "candidate_status": candidate["status"],
                "reference_matches_expected": (
                    case["expected_status"] is None
                    or (reference["status"] or "").lower()
                    == case["expected_status"].lower()
                ),
                "candidate_matches_expected": (
                    case["expected_status"] is None
                    or (candidate["status"] or "").lower()
                    == case["expected_status"].lower()
                ),
                "reference_exit_code": reference["exit_code"],
                "candidate_exit_code": candidate["exit_code"],
                "reference_seconds": reference["wall_seconds"],
                "candidate_seconds": candidate["wall_seconds"],
                "normalized_output_equal": normalized_output_equal,
                "mismatches": mismatches,
                "expected_mismatches": expected_mismatches,
                "mismatch_expectation_met": mismatch_expectation_met,
            }
        )

    summary = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "candidate_kind": candidate_kind,
        "reference_manifest": manifest,
        "timeout_seconds": args.timeout,
        "memory_limit_mb": args.memory_limit_mb,
        "case_count": len(records),
        "mismatch_count": mismatch_count,
        "expected_difference_count": expected_difference_count,
        "cases": records,
    }
    write_json(run_dir / "comparison.json", summary)
    write_csv(run_dir / "comparison.csv", records)
    print(f"Comparison report: {run_dir}")
    print(
        f"Cases: {len(records)}; mismatches: {mismatch_count}; "
        f"expected differences: {expected_difference_count}"
    )
    if mismatch_count and not args.report_only:
        raise InteropError("Compatibility mismatches were found")


def tool_comparison_cases(tool_names: Sequence[str]) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for tool in sorted(tool_names):
        for arguments in tool_argument_cases(tool):
            label = "-".join(part.strip("-") or "dash" for part in arguments)
            cases.append(
                {
                    "tool": tool,
                    "name": f"{tool}/{label}",
                    "arguments": list(arguments),
                    "scenario": label,
                    "stdin": None,
                }
            )
        for functional_case in TOOL_FUNCTIONAL_CASES.get(tool, ()):
            label, arguments, stdin_text, *fixture_tail = functional_case
            metadata = tool_functional_case_metadata(fixture_tail)
            cases.append(
                {
                    "tool": tool,
                    "name": f"{tool}/{label}",
                    "arguments": list(arguments),
                    "scenario": label,
                    "stdin": stdin_text,
                    **metadata,
                }
            )
    return cases


def tool_functional_case_metadata(fixture_tail: Sequence[Any]) -> dict[str, Any]:
    if not fixture_tail:
        return {
            "fixture_files": {},
            "isolated_workdir": False,
            "workdir_files": {},
            "workdir_directories": [],
            "output_files": [],
            "output_absent_files": [],
            "output_directories": [],
            "normalize_legacy_classify_feature_suffix": False,
            "expected_mismatches": [],
            "reference_mode": "fol",
        }
    if len(fixture_tail) != 1:
        raise InteropError("Functional support-tool cases accept at most one metadata argument")

    tail = fixture_tail[0]
    if not isinstance(tail, dict):
        raise InteropError("Functional support-tool case metadata must be a dictionary")
    if any(key in TOOL_CASE_METADATA_KEYS for key in tail):
        unknown_keys = sorted(set(tail) - TOOL_CASE_METADATA_KEYS)
        if unknown_keys:
            raise InteropError(
                "Unknown functional support-tool case metadata key(s): "
                + ", ".join(unknown_keys)
            )
        fixture_files = tail.get("fixture_files", {})
        isolated_workdir = tail.get("isolated_workdir", False)
        workdir_files = tail.get("workdir_files", {})
        workdir_directories = tail.get("workdir_directories", ())
        output_files = tail.get("output_files", ())
        output_absent_files = tail.get("output_absent_files", ())
        output_directories = tail.get("output_directories", ())
        normalize_legacy_classify_feature_suffix = tail.get(
            "normalize_legacy_classify_feature_suffix", False
        )
        expected_mismatches = tail.get("expected_mismatches", ())
        reference_mode = tail.get("reference_mode", "fol")
        if reference_mode not in {"fol", "ho"}:
            raise InteropError("Functional support-tool reference_mode must be 'fol' or 'ho'")
        if isinstance(expected_mismatches, (str, bytes)):
            raise InteropError(
                "Functional support-tool expected_mismatches must be a sequence"
            )
        unknown_mismatches = sorted(
            set(expected_mismatches) - TOOL_COMPARISON_MISMATCH_FIELDS
        )
        if unknown_mismatches:
            raise InteropError(
                "Unknown functional support-tool expected mismatch(es): "
                + ", ".join(unknown_mismatches)
            )
    else:
        fixture_files = tail
        isolated_workdir = False
        workdir_files = {}
        workdir_directories = ()
        output_files = ()
        output_absent_files = ()
        output_directories = ()
        normalize_legacy_classify_feature_suffix = False
        expected_mismatches = ()
        reference_mode = "fol"

    return {
        "fixture_files": dict(fixture_files),
        "isolated_workdir": bool(isolated_workdir),
        "workdir_files": dict(workdir_files),
        "workdir_directories": list(workdir_directories),
        "output_files": list(output_files),
        "output_absent_files": list(output_absent_files),
        "output_directories": list(output_directories),
        "normalize_legacy_classify_feature_suffix": bool(
            normalize_legacy_classify_feature_suffix
        ),
        "expected_mismatches": list(expected_mismatches),
        "reference_mode": reference_mode,
    }


def reference_tool_binary(
    manifest: dict[str, Any], reference_tools: dict[str, str], tool: str, mode: str
) -> Path:
    if mode == "fol":
        return Path(reference_tools[tool])
    build = manifest["builds"].get(mode)
    relative = REFERENCE_TOOL_BINARIES.get(tool)
    if build is None or relative is None:
        raise InteropError(f"No {mode} reference build is available for tool {tool}")
    binary = Path(build["build_source"]) / relative
    if not binary.is_file():
        raise InteropError(
            f"Expected {mode} reference tool was not built: {binary}. "
            "Run build-reference again."
        )
    return binary


def validate_tool_relative_name(name: str, kind: str) -> Path:
    relative = Path(name)
    if relative.is_absolute() or not relative.parts or any(part == ".." for part in relative.parts):
        raise InteropError(f"Invalid {kind} name: {name}")
    return relative


def validate_tool_fixture_name(name: str) -> Path:
    return validate_tool_relative_name(name, "fixture file")


def validate_tool_output_name(name: str) -> Path:
    return validate_tool_relative_name(name, "output file")


def validate_tool_workdir_directory_name(name: str) -> Path:
    return validate_tool_relative_name(name, "workdir directory")


def validate_tool_output_directory_name(name: str) -> Path:
    return validate_tool_relative_name(name, "output directory")


def materialize_tool_named_files(files: dict[str, str], directory: Path, kind: str) -> dict[str, Path]:
    paths: dict[str, Path] = {}
    for name, contents in files.items():
        relative = validate_tool_relative_name(name, kind)
        path = directory / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")
        paths[name] = path
    return paths


def materialize_tool_fixture_files(case: dict[str, Any], fixture_dir: Path) -> dict[str, Path]:
    return materialize_tool_named_files(
        case.get("fixture_files", {}), fixture_dir, "fixture file"
    )


def materialize_tool_workdir_files(case: dict[str, Any], workdir: Path) -> dict[str, Path]:
    return materialize_tool_named_files(
        case.get("workdir_files", {}), workdir, "workdir file"
    )


def materialize_tool_workdir_directories(case: dict[str, Any], workdir: Path) -> list[Path]:
    paths: list[Path] = []
    for name in case.get("workdir_directories", ()):
        relative = validate_tool_workdir_directory_name(name)
        path = workdir / relative
        path.mkdir(parents=True, exist_ok=True)
        paths.append(path)
    return paths


def substitute_tool_fixture_arguments(
    arguments: Sequence[str],
    fixture_paths: dict[str, Path],
) -> list[str]:
    def replacement(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in fixture_paths:
            raise InteropError(f"Unknown fixture placeholder: {name}")
        return str(fixture_paths[name])

    return [FIXTURE_ARGUMENT_RE.sub(replacement, argument) for argument in arguments]


def substitute_tool_companion_arguments(
    arguments: Sequence[str],
    companion_paths: dict[str, Path],
) -> list[str]:
    def replacement(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in companion_paths:
            raise InteropError(f"Unknown companion placeholder: {name}")
        path = companion_paths[name]
        if not path.is_file():
            raise InteropError(f"Companion binary is missing: {path}")
        return str(path)

    return [COMPANION_ARGUMENT_RE.sub(replacement, argument) for argument in arguments]


def compare_tool_output_files(
    output_files: Sequence[str],
    reference_cwd: Path,
    candidate_cwd: Path,
    replacements: Iterable[tuple[str, str]],
    *,
    normalize_legacy_classify_feature_suffix: bool = False,
) -> tuple[list[dict[str, Any]], dict[str, dict[str, Any]]]:
    records: list[dict[str, Any]] = []
    details: dict[str, dict[str, Any]] = {}
    replacement_list = list(replacements)
    for name in output_files:
        relative = validate_tool_output_name(name)
        reference_path = reference_cwd / relative
        candidate_path = candidate_cwd / relative
        reference_exists = reference_path.is_file()
        candidate_exists = candidate_path.is_file()
        reference_text = (
            reference_path.read_text(encoding="utf-8", errors="replace")
            if reference_exists
            else None
        )
        candidate_text = (
            candidate_path.read_text(encoding="utf-8", errors="replace")
            if candidate_exists
            else None
        )
        reference_normalized = (
            normalize_output(
                reference_text,
                replacement_list,
                normalize_legacy_classify_feature_suffix=(
                    normalize_legacy_classify_feature_suffix
                ),
            )
            if reference_text is not None
            else None
        )
        candidate_normalized = (
            normalize_output(
                candidate_text,
                replacement_list,
                normalize_legacy_classify_feature_suffix=(
                    normalize_legacy_classify_feature_suffix
                ),
            )
            if candidate_text is not None
            else None
        )
        normalized_equal = (
            reference_normalized is not None
            and candidate_normalized is not None
            and reference_normalized == candidate_normalized
        )
        records.append(
            {
                "name": name,
                "reference_exists": reference_exists,
                "candidate_exists": candidate_exists,
                "normalized_equal": normalized_equal,
            }
        )
        details[name] = {
            "relative": relative,
            "reference_text": reference_text,
            "candidate_text": candidate_text,
            "reference_normalized": reference_normalized,
            "candidate_normalized": candidate_normalized,
            "normalized_equal": normalized_equal,
        }
    return records, details


def compare_tool_absent_output_files(
    output_absent_files: Sequence[str],
    reference_cwd: Path,
    candidate_cwd: Path,
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for name in output_absent_files:
        relative = validate_tool_output_name(name)
        reference_exists = (reference_cwd / relative).exists()
        candidate_exists = (candidate_cwd / relative).exists()
        records.append(
            {
                "name": name,
                "reference_absent": not reference_exists,
                "candidate_absent": not candidate_exists,
                "absent_equal": not reference_exists and not candidate_exists,
            }
        )
    return records


def compare_tool_output_directories(
    output_directories: Sequence[str],
    reference_cwd: Path,
    candidate_cwd: Path,
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for name in output_directories:
        relative = validate_tool_output_directory_name(name)
        reference_exists = (reference_cwd / relative).is_dir()
        candidate_exists = (candidate_cwd / relative).is_dir()
        records.append(
            {
                "name": name,
                "reference_exists": reference_exists,
                "candidate_exists": candidate_exists,
                "equal": reference_exists and candidate_exists,
            }
        )
    return records


def tool_argument_cases(tool: str) -> tuple[tuple[str, ...], ...]:
    if tool in VERSIONED_REFERENCE_TOOLS:
        return (*DEFAULT_TOOL_ARGUMENT_CASES, ("--version",))
    return DEFAULT_TOOL_ARGUMENT_CASES


def compare_tools(args: argparse.Namespace) -> None:
    manifest = load_manifest()
    repo_root = args.repo_root.resolve()
    reference_tools = manifest["builds"]["fol"].get("tools", {})
    if not reference_tools:
        raise InteropError(
            "Reference manifest has no support-tool binaries. Run build-reference again."
        )

    selected = args.tool or sorted(reference_tools)
    unknown = sorted(set(selected) - set(reference_tools))
    if unknown:
        raise InteropError("Unknown reference tool(s): " + ", ".join(unknown))

    output_root = artifact_root() / "tools"
    run_id = dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f") + "-tools"
    run_dir = output_root / run_id
    run_dir.mkdir(parents=True, exist_ok=False)

    if args.self_test:
        candidate_kind = "linux-reference-tools"
        rust_bin_dir = None
    else:
        rust_bin_dir = args.rust_bin_dir.resolve()
        if not rust_bin_dir.is_dir():
            raise InteropError(f"Native Linux Rust bin directory not found: {rust_bin_dir}")
        candidate_kind = "linux-rust-tools"

    records: list[dict[str, Any]] = []
    mismatch_count = 0
    expected_difference_count = 0
    reference_companions = {
        "eprover": Path(manifest["builds"]["fol"]["binary"]),
    }
    candidate_companions = reference_companions if args.self_test else {
        "eprover": rust_bin_dir / "eprover",
    }
    cases = tool_comparison_cases(selected)
    for index, case in enumerate(cases, 1):
        tool = case["tool"]
        print(f"[{index}/{len(cases)}] {case['name']}", flush=True)
        reference_mode = case.get("reference_mode", "fol")
        reference_binary = reference_tool_binary(
            manifest, reference_tools, tool, reference_mode
        )
        uses_case_workdir = bool(
            case.get("isolated_workdir")
            or case.get("workdir_files")
            or case.get("workdir_directories")
            or case.get("output_files")
            or case.get("output_absent_files")
            or case.get("output_directories")
        )
        if uses_case_workdir:
            case_workdir_root = run_dir / "workdirs" / f"{index:04d}"
            reference_cwd = case_workdir_root / "reference"
            candidate_cwd = case_workdir_root / "candidate"
            reference_cwd.mkdir(parents=True, exist_ok=False)
            candidate_cwd.mkdir(parents=True, exist_ok=False)
            reference_workdir_directories = materialize_tool_workdir_directories(
                case, reference_cwd
            )
            candidate_workdir_directories = materialize_tool_workdir_directories(
                case, candidate_cwd
            )
            reference_workdir_paths = materialize_tool_workdir_files(case, reference_cwd)
            candidate_workdir_paths = materialize_tool_workdir_files(case, candidate_cwd)
        else:
            reference_cwd = repo_root
            candidate_cwd = repo_root
            reference_workdir_directories = []
            candidate_workdir_directories = []
            reference_workdir_paths = {}
            candidate_workdir_paths = {}
        fixture_paths = materialize_tool_fixture_files(
            case, run_dir / "fixtures" / f"{index:04d}"
        )
        reference_arguments = substitute_tool_fixture_arguments(case["arguments"], fixture_paths)
        reference_arguments = substitute_tool_companion_arguments(
            reference_arguments, reference_companions
        )
        environment = os.environ.copy()
        reference = execute(
            reference_binary,
            reference_arguments,
            timeout=args.timeout,
            env=environment,
            stdin_text=case["stdin"],
            cwd=reference_cwd,
        )

        if args.self_test:
            candidate_binary = reference_binary
        else:
            assert rust_bin_dir is not None
            candidate_binary = rust_bin_dir / tool
            if not candidate_binary.is_file():
                raise InteropError(f"Native Linux Rust tool executable not found: {candidate_binary}")
        candidate_arguments = substitute_tool_fixture_arguments(
            case["arguments"], fixture_paths
        )
        candidate_arguments = substitute_tool_companion_arguments(
            candidate_arguments,
            candidate_companions,
        )

        candidate = execute(
            candidate_binary,
            candidate_arguments,
            timeout=args.timeout,
            env=environment,
            stdin_text=case["stdin"],
            cwd=candidate_cwd,
        )
        mismatches = comparison_mismatches(reference, candidate)
        fixture_replacements: list[tuple[str, str]] = []
        fixture_replacements.extend(
            tool_binary_path_replacements(
                reference_binary,
                candidate_binary,
                tool,
            )
        )
        for fixture_path in fixture_paths.values():
            fixture_replacements.extend(
                cross_platform_path_replacements(fixture_path, "<FIXTURE>")
            )
        if uses_case_workdir:
            for workdir in (reference_cwd, candidate_cwd):
                fixture_replacements.extend(
                    cross_platform_path_replacements(workdir, "<WORKDIR>")
                )
            for workdir_path in (
                *reference_workdir_paths.values(),
                *candidate_workdir_paths.values(),
                *reference_workdir_directories,
                *candidate_workdir_directories,
            ):
                fixture_replacements.extend(
                    cross_platform_path_replacements(workdir_path, "<WORKDIR_FILE>")
                )
        normalize_legacy_classify_feature_suffix = case.get(
            "normalize_legacy_classify_feature_suffix", False
        )
        reference_normalized_stdout = normalize_output(
            reference["stdout"],
            fixture_replacements,
            normalize_legacy_classify_feature_suffix=(
                normalize_legacy_classify_feature_suffix
            ),
        )
        candidate_normalized_stdout = normalize_output(
            candidate["stdout"],
            fixture_replacements,
            normalize_legacy_classify_feature_suffix=(
                normalize_legacy_classify_feature_suffix
            ),
        )
        reference_normalized_stderr = normalize_output(
            reference["stderr"], fixture_replacements
        )
        candidate_normalized_stderr = normalize_output(
            candidate["stderr"], fixture_replacements
        )
        normalized_stdout_equal = reference_normalized_stdout == candidate_normalized_stdout
        normalized_stderr_equal = reference_normalized_stderr == candidate_normalized_stderr
        if not normalized_stdout_equal:
            mismatches.append("normalized_stdout")
        if not normalized_stderr_equal:
            mismatches.append("normalized_stderr")
        output_file_records, output_file_details = compare_tool_output_files(
            case.get("output_files", ()),
            reference_cwd,
            candidate_cwd,
            fixture_replacements,
            normalize_legacy_classify_feature_suffix=(
                normalize_legacy_classify_feature_suffix
            ),
        )
        output_files_equal = all(record["normalized_equal"] for record in output_file_records)
        if not output_files_equal:
            mismatches.append("output_files")
        output_absent_file_records = compare_tool_absent_output_files(
            case.get("output_absent_files", ()),
            reference_cwd,
            candidate_cwd,
        )
        output_absent_files_equal = all(
            record["absent_equal"] for record in output_absent_file_records
        )
        if not output_absent_files_equal:
            mismatches.append("output_absent_files")
        output_directory_records = compare_tool_output_directories(
            case.get("output_directories", ()),
            reference_cwd,
            candidate_cwd,
        )
        output_directories_equal = all(
            record["equal"] for record in output_directory_records
        )
        if not output_directories_equal:
            mismatches.append("output_directories")

        expected_mismatches = (
            [] if args.self_test else list(case.get("expected_mismatches", ()))
        )
        mismatch_expectation_met = mismatch_expectation_matches(
            mismatches, expected_mismatches
        )
        if mismatches and mismatch_expectation_met:
            expected_difference_count += 1
        if not mismatch_expectation_met:
            mismatch_count += 1
        if mismatches or not mismatch_expectation_met:
            difference_kind = (
                "expected-differences" if mismatch_expectation_met else "mismatches"
            )
            mismatch_dir = run_dir / difference_kind / f"{index:04d}"
            mismatch_dir.mkdir(parents=True, exist_ok=True)
            for label, result in (("reference", reference), ("candidate", candidate)):
                (mismatch_dir / f"{label}.stdout").write_text(result["stdout"], encoding="utf-8")
                (mismatch_dir / f"{label}.stderr").write_text(result["stderr"], encoding="utf-8")
            (mismatch_dir / "reference.normalized.stdout").write_text(
                reference_normalized_stdout, encoding="utf-8"
            )
            (mismatch_dir / "candidate.normalized.stdout").write_text(
                candidate_normalized_stdout, encoding="utf-8"
            )
            (mismatch_dir / "reference.normalized.stderr").write_text(
                reference_normalized_stderr, encoding="utf-8"
            )
            (mismatch_dir / "candidate.normalized.stderr").write_text(
                candidate_normalized_stderr, encoding="utf-8"
            )
            for name, detail in output_file_details.items():
                if detail["normalized_equal"]:
                    continue
                output_path = mismatch_dir / "output-files" / detail["relative"]
                output_path.parent.mkdir(parents=True, exist_ok=True)
                reference_text = detail["reference_text"]
                candidate_text = detail["candidate_text"]
                reference_normalized = detail["reference_normalized"]
                candidate_normalized = detail["candidate_normalized"]
                (output_path.with_name(output_path.name + ".reference")).write_text(
                    reference_text if reference_text is not None else "<missing>\n",
                    encoding="utf-8",
                )
                (output_path.with_name(output_path.name + ".candidate")).write_text(
                    candidate_text if candidate_text is not None else "<missing>\n",
                    encoding="utf-8",
                )
                (output_path.with_name(output_path.name + ".reference.normalized")).write_text(
                    reference_normalized
                    if reference_normalized is not None
                    else "<missing>\n",
                    encoding="utf-8",
                )
                (output_path.with_name(output_path.name + ".candidate.normalized")).write_text(
                    candidate_normalized
                    if candidate_normalized is not None
                    else "<missing>\n",
                    encoding="utf-8",
                )

        records.append(
            {
                "name": case["name"],
                "tool": tool,
                "reference_mode": reference_mode,
                "scenario": case["scenario"],
                "arguments": case["arguments"],
                "stdin": case["stdin"] is not None,
                "fixtures": bool(fixture_paths),
                "isolated_workdir": bool(case.get("isolated_workdir")),
                "workdir_files": bool(reference_workdir_paths),
                "workdir_directories": bool(reference_workdir_directories),
                "output_files": output_file_records,
                "output_files_equal": output_files_equal,
                "output_absent_files": output_absent_file_records,
                "output_absent_files_equal": output_absent_files_equal,
                "output_directories": output_directory_records,
                "output_directories_equal": output_directories_equal,
                "reference_exit_code": reference["exit_code"],
                "candidate_exit_code": candidate["exit_code"],
                "reference_seconds": reference["wall_seconds"],
                "candidate_seconds": candidate["wall_seconds"],
                "normalized_stdout_equal": normalized_stdout_equal,
                "normalized_stderr_equal": normalized_stderr_equal,
                "mismatches": mismatches,
                "expected_mismatches": expected_mismatches,
                "mismatch_expectation_met": mismatch_expectation_met,
            }
        )

    summary = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "candidate_kind": candidate_kind,
        "reference_manifest": manifest,
        "timeout_seconds": args.timeout,
        "case_count": len(records),
        "mismatch_count": mismatch_count,
        "expected_difference_count": expected_difference_count,
        "cases": records,
    }
    write_json(run_dir / "tool-comparison.json", summary)
    write_csv(run_dir / "tool-comparison.csv", records)
    print(f"Tool comparison report: {run_dir}")
    print(
        f"Cases: {len(records)}; mismatches: {mismatch_count}; "
        f"expected differences: {expected_difference_count}"
    )
    if mismatch_count and not args.report_only:
        raise InteropError("Support-tool compatibility mismatches were found")


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        return
    fields = list(rows[0].keys())
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            encoded = dict(row)
            for key, value in encoded.items():
                if isinstance(value, (list, dict)):
                    encoded[key] = json.dumps(value, sort_keys=True)
            writer.writerow(encoded)


def stage_benchmark_corpus(
    repo_root: Path, corpus: Path | None, commit: str
) -> tuple[Path, list[dict[str, Any]]]:
    destination = cache_root() / "benchmark-corpus" / commit
    if destination.exists():
        safe_remove_cache_path(destination)
    destination.mkdir(parents=True)
    if corpus:
        staged = destination / "custom"
        shutil.copytree(corpus, staged)
        cases = enumerate_problems(repo_root, staged)
    else:
        source = repo_root / "eprover" / "EXAMPLE_PROBLEMS"
        staged = destination / "EXAMPLE_PROBLEMS"
        shutil.copytree(source, staged)
        cases = enumerate_problems(repo_root, staged / "SMOKETEST")
    return staged, cases


def timed_execution(
    executable: Path,
    arguments: Sequence[str],
    *,
    timeout: int,
    env: dict[str, str],
    cwd: Path,
) -> dict[str, Any]:
    with tempfile.NamedTemporaryFile(prefix="e-time-", delete=False) as handle:
        metrics_path = Path(handle.name)
    command = [
        "/usr/bin/time",
        "-f",
        "%U,%S,%M",
        "-o",
        str(metrics_path),
        str(executable),
        *map(str, arguments),
    ]
    started = time.perf_counter()
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            text=True,
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        timed_out = False
        exit_code: int | None = result.returncode
    except subprocess.TimeoutExpired:
        timed_out = True
        exit_code = None
        result = None
    wall = time.perf_counter() - started
    user = system = max_rss = None
    if metrics_path.exists():
        metrics = metrics_path.read_text(encoding="utf-8", errors="replace").strip().splitlines()
        if metrics:
            values = metrics[-1].split(",")
            if len(values) == 3:
                try:
                    user, system, max_rss = float(values[0]), float(values[1]), int(values[2])
                except ValueError:
                    pass
        metrics_path.unlink(missing_ok=True)
    stdout = result.stdout if result else ""
    return {
        "exit_code": exit_code,
        "timed_out": timed_out,
        "wall_seconds": wall,
        "cpu_seconds": (user + system) if user is not None and system is not None else None,
        "max_rss_kb": max_rss,
        "status": szs_status(stdout),
    }


def geometric_mean(values: Sequence[float]) -> float | None:
    positive = [value for value in values if value > 0 and math.isfinite(value)]
    if not positive:
        return None
    return math.exp(sum(math.log(value) for value in positive) / len(positive))


def benchmark(args: argparse.Namespace) -> None:
    manifest = load_manifest()
    repo_root = args.repo_root.resolve()
    commit = manifest["upstream_commit"]
    rust_binary = args.rust_bin.resolve()
    if not rust_binary.is_file():
        raise InteropError(f"Native Linux Rust executable not found: {rust_binary}")
    rust_environment = rust_tool_environment()
    rust_metadata = {
        "cargo": first_line(["cargo", "--version"], env=rust_environment),
        "rustc": first_line(["rustc", "--version"], env=rust_environment),
        "sha256": sha256_file(rust_binary),
    }
    corpus = args.corpus.resolve() if args.corpus else None
    staged_root, cases = stage_benchmark_corpus(repo_root, corpus, commit)
    output_root = artifact_root() / "benchmark"
    run_dir = output_root / (
        dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f") + "-benchmark"
    )
    run_dir.mkdir(parents=True, exist_ok=False)

    # Warm both binaries on the first problem. Warmup data is intentionally discarded.
    warm = cases[0]
    warm_tptp = staged_root if corpus else staged_root / "TPTP"
    warm_env = os.environ.copy()
    warm_env["TPTP"] = str(warm_tptp)
    options = common_arguments(args.timeout, args.memory_limit_mb, proof=False)
    for binary in (
        Path(manifest["builds"][warm["mode"]]["binary"]),
        rust_binary,
    ):
        timed_execution(
            binary,
            [str(warm["path"]), *options],
            timeout=args.timeout + 10,
            env=warm_env,
            cwd=warm["path"].parent,
        )

    rng = random.Random(0)
    samples: list[dict[str, Any]] = []
    for case_index, case in enumerate(cases, 1):
        print(f"[{case_index}/{len(cases)}] {case['name']}", flush=True)
        tptp = staged_root if corpus else staged_root / "TPTP"
        environment = os.environ.copy()
        environment["TPTP"] = str(tptp)
        binaries = {
            "c": Path(manifest["builds"][case["mode"]]["binary"]),
            "rust": rust_binary,
        }
        for iteration in range(args.runs):
            order = ["c", "rust"]
            rng.shuffle(order)
            for implementation in order:
                result = timed_execution(
                    binaries[implementation],
                    [str(case["path"]), *options],
                    timeout=args.timeout + 10,
                    env=environment,
                    cwd=case["path"].parent,
                )
                samples.append(
                    {
                        "name": case["name"],
                        "mode": case["mode"],
                        "iteration": iteration + 1,
                        "implementation": implementation,
                        **result,
                    }
                )

    summaries: list[dict[str, Any]] = []
    ratios: list[float] = []
    for case in cases:
        row: dict[str, Any] = {"name": case["name"], "mode": case["mode"]}
        grouped = {
            implementation: [
                sample
                for sample in samples
                if sample["name"] == case["name"]
                and sample["implementation"] == implementation
            ]
            for implementation in ("c", "rust")
        }
        for implementation, values in grouped.items():
            row[f"{implementation}_median_wall_seconds"] = statistics.median(
                value["wall_seconds"] for value in values
            )
            cpu_values = [value["cpu_seconds"] for value in values if value["cpu_seconds"] is not None]
            rss_values = [value["max_rss_kb"] for value in values if value["max_rss_kb"] is not None]
            row[f"{implementation}_median_cpu_seconds"] = (
                statistics.median(cpu_values) if cpu_values else None
            )
            row[f"{implementation}_max_rss_kb"] = max(rss_values) if rss_values else None
        c_wall = row["c_median_wall_seconds"]
        rust_wall = row["rust_median_wall_seconds"]
        c_outcomes = {(value["exit_code"], value["timed_out"], value["status"]) for value in grouped["c"]}
        rust_outcomes = {(value["exit_code"], value["timed_out"], value["status"]) for value in grouped["rust"]}
        behavior_matches = len(c_outcomes) == 1 and c_outcomes == rust_outcomes
        row["behavior_matches"] = behavior_matches
        ratio = rust_wall / c_wall if behavior_matches and c_wall > 0 else None
        row["rust_to_c_wall_ratio"] = ratio
        row["regression_over_threshold"] = bool(
            ratio is not None and ratio > args.regression_threshold
        )
        if ratio is not None:
            ratios.append(ratio)
        summaries.append(row)

    aggregate_ratio = geometric_mean(ratios)
    behavior_mismatch_count = sum(not row["behavior_matches"] for row in summaries)
    report = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "reference_manifest": manifest,
        "rust": rust_metadata,
        "runs": args.runs,
        "seed": 0,
        "timeout_seconds": args.timeout,
        "memory_limit_mb": args.memory_limit_mb,
        "regression_threshold": args.regression_threshold,
        "aggregate_rust_to_c_wall_ratio": aggregate_ratio,
        "behavior_mismatch_count": behavior_mismatch_count,
        "regression_over_threshold": bool(
            aggregate_ratio is not None and aggregate_ratio > args.regression_threshold
        ),
        "cases": summaries,
        "samples": samples,
    }
    write_json(run_dir / "benchmark.json", report)
    write_csv(run_dir / "benchmark.csv", summaries)
    write_csv(run_dir / "benchmark-samples.csv", samples)
    print(f"Benchmark report: {run_dir}")
    if aggregate_ratio is not None:
        print(f"Aggregate Rust/C wall-time ratio: {aggregate_ratio:.3f}x")
        if aggregate_ratio > args.regression_threshold:
            print(
                f"WARNING: ratio exceeds the {args.regression_threshold:.3f}x regression threshold",
                file=sys.stderr,
            )
    if behavior_mismatch_count:
        print(
            f"WARNING: {behavior_mismatch_count} benchmark cases had differing outcomes; "
            "their timing ratios were excluded",
            file=sys.stderr,
        )


def doctor(_: argparse.Namespace) -> None:
    missing = [
        tool
        for tool in ("gcc", "gawk", "git", "make", "python3", "/usr/bin/time")
        if shutil.which(tool) is None
    ]
    release = os_release()
    if missing:
        raise InteropError("Missing Linode worker dependencies: " + ", ".join(missing))
    print(f"Distribution: {release.get('PRETTY_NAME', 'unknown')}")
    if release.get("ID") != "ubuntu" or release.get("VERSION_ID") != "24.04":
        print(
            f"WARNING: tested baseline is {DEFAULT_DISTRO}; found "
            f"{release.get('ID', 'unknown')} {release.get('VERSION_ID', 'unknown')}",
            file=sys.stderr,
        )
    print(first_line(["gcc", "--version"]))
    print(first_line(["python3", "--version"]))


def path_argument(value: str) -> Path:
    return Path(value)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    subparsers = result.add_subparsers(dest="command", required=True)

    doctor_parser = subparsers.add_parser(
        "doctor", help="validate native Linux worker dependencies"
    )
    doctor_parser.set_defaults(function=doctor)

    build_parser = subparsers.add_parser("build-reference", help="build FOL and HO E references")
    build_parser.add_argument("--repo-root", type=path_argument, required=True)
    build_parser.add_argument("--eprover-commit", required=True)
    build_parser.set_defaults(function=build_reference)

    compare_parser = subparsers.add_parser(
        "compare", help="compare native Linux C and Rust eprover binaries"
    )
    compare_parser.add_argument("--repo-root", type=path_argument, required=True)
    candidate = compare_parser.add_mutually_exclusive_group(required=True)
    candidate.add_argument("--rust-bin", type=path_argument)
    candidate.add_argument("--self-test", action="store_true")
    compare_parser.add_argument("--corpus", type=path_argument)
    compare_parser.add_argument("--timeout", type=int, default=60)
    compare_parser.add_argument("--memory-limit-mb", type=int, default=2048)
    compare_parser.add_argument("--report-only", action="store_true")
    compare_parser.set_defaults(function=compare)

    compare_tools_parser = subparsers.add_parser(
        "compare-tools", help="compare native Linux C and Rust support tools"
    )
    compare_tools_parser.add_argument("--repo-root", type=path_argument, required=True)
    tool_candidate = compare_tools_parser.add_mutually_exclusive_group(required=True)
    tool_candidate.add_argument("--rust-bin-dir", type=path_argument)
    tool_candidate.add_argument("--self-test", action="store_true")
    compare_tools_parser.add_argument(
        "--tool",
        action="append",
        help="support tool to compare; may be repeated; defaults to all archived tools",
    )
    compare_tools_parser.add_argument("--timeout", type=int, default=30)
    compare_tools_parser.add_argument("--report-only", action="store_true")
    compare_tools_parser.set_defaults(function=compare_tools)

    benchmark_parser = subparsers.add_parser("benchmark", help="benchmark native Linux C and Rust binaries")
    benchmark_parser.add_argument("--repo-root", type=path_argument, required=True)
    benchmark_parser.add_argument("--rust-bin", type=path_argument, required=True)
    benchmark_parser.add_argument("--corpus", type=path_argument)
    benchmark_parser.add_argument("--runs", type=int, default=5)
    benchmark_parser.add_argument("--timeout", type=int, default=60)
    benchmark_parser.add_argument("--memory-limit-mb", type=int, default=2048)
    benchmark_parser.add_argument("--regression-threshold", type=float, default=1.10)
    benchmark_parser.set_defaults(function=benchmark)
    return result


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        args.function(args)
        return 0
    except (InteropError, OSError, KeyError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
