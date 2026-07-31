#!/usr/bin/env python3
"""Focused tests for the IPASIR-UP-style simulation."""

from __future__ import annotations

import unittest

import simulator


class SimulatorTests(unittest.TestCase):
    def test_generated_corpus_is_deterministic_and_balanced(self) -> None:
        first = simulator.generated_cases(count=10)
        second = simulator.generated_cases(count=10)
        self.assertEqual(first, second)
        self.assertEqual(sum(case.expected for case in first), 5)

    def test_oracle_matches_pigeonhole_criterion(self) -> None:
        for case in (*simulator.hand_cases(), *simulator.generated_cases(count=10)):
            self.assertEqual(simulator.exhaustive_oracle(case), case.expected)

    def test_conflict_reason_is_valid(self) -> None:
        case = simulator.hand_cases()[1]
        event = simulator.LearnEvent(
            "conflict",
            (-1, -3),
            None,
            ((1, True), (3, True)),
            (
                simulator.TrailEntry(1, 1, "decision"),
                simulator.TrailEntry(3, 2, "decision"),
            ),
        )
        self.assertTrue(simulator.validate_reason(case, event))

    def test_propagation_reason_is_unit_and_contains_literal(self) -> None:
        case = simulator.hand_cases()[0]
        event = simulator.LearnEvent(
            "propagation",
            (-2, -1),
            -2,
            ((1, True),),
            (simulator.TrailEntry(1, 1, "decision"),),
        )
        self.assertTrue(simulator.validate_reason(case, event))
        self.assertFalse(
            simulator.validate_reason(
                case, dataclasses_replace(event, propagated=-4)
            )
        )

    def test_invalid_cross_group_reason_is_rejected(self) -> None:
        case = simulator.hand_cases()[0]
        event = simulator.LearnEvent(
            "conflict",
            (-1, -4),
            None,
            ((1, True), (4, True)),
            (),
        )
        self.assertFalse(simulator.validate_reason(case, event))

    def test_all_treatments_agree(self) -> None:
        for case in (*simulator.hand_cases(), *simulator.generated_cases(count=6)):
            for treatment in simulator.Treatment:
                outcome = simulator.run_case(case, treatment)
                self.assertEqual(outcome.decision, case.expected)
                self.assertLess(outcome.metrics.steps, 1_000_000)

    def test_backtracks_are_rooted_and_empty(self) -> None:
        case = simulator.hand_cases()[1]
        for treatment in (
            simulator.Treatment.LAZY,
            simulator.Treatment.CONFLICT,
            simulator.Treatment.PROPAGATE,
        ):
            outcome = simulator.run_case(case, treatment)
            for event in outcome.events:
                self.assertEqual(event["backtrack"]["to_level"], 0)
                self.assertEqual(event["backtrack"]["post_trail"], [])

    def test_backtrack_mutations_are_rejected(self) -> None:
        event = simulator.LearnEvent(
            "conflict",
            (-1, -3),
            None,
            ((1, True), (3, True)),
            (
                simulator.TrailEntry(1, 1, "decision"),
                simulator.TrailEntry(3, 2, "decision"),
            ),
        )
        self.assertTrue(simulator.validate_root_backtrack(event, 0, ()))
        self.assertFalse(simulator.validate_root_backtrack(event, 1, ()))
        self.assertFalse(
            simulator.validate_root_backtrack(
                event, 0, (simulator.TrailEntry(1, 1, "stale"),)
            )
        )

    def test_semantic_trace_is_deterministic(self) -> None:
        case = simulator.generated_cases(count=2)[1]
        first = simulator.run_case(case, "propagate")
        second = simulator.run_case(case, "propagate")
        self.assertEqual(first.semantic_sha256, second.semantic_sha256)
        self.assertEqual(first.metrics.semantic(), second.metrics.semantic())


def dataclasses_replace(value: simulator.LearnEvent, **changes: object) -> simulator.LearnEvent:
    import dataclasses

    return dataclasses.replace(value, **changes)


if __name__ == "__main__":
    unittest.main()
