#!/usr/bin/env python3
"""Falsification tests for the persistent SATCheck lifecycle model."""

from __future__ import annotations

import unittest

from campaign import run_campaign
from model import (
    CapacityError,
    FailurePlan,
    InjectedBackendFailure,
    PersistentSatModel,
    SessionPoisoned,
    atom,
    fresh_satisfiable,
    negative,
    positive,
)


A = atom("a")
B = atom("b")
C = atom("c")
D = atom("d")
ATOMS = (A, B, C, D)


class PersistentSatModelTests(unittest.TestCase):
    def assert_agrees(self, session: PersistentSatModel, snapshot: dict) -> None:
        result = session.solve()
        self.assertEqual(result.satisfiable, fresh_satisfiable(snapshot))
        self.assertEqual(result.active_sources, tuple(sorted(session.active_by_source)))
        if not result.satisfiable:
            self.assertTrue(result.core_sources)
            core = {
                source: session.active_by_source[source].literals
                for source in result.core_sources
            }
            self.assertFalse(fresh_satisfiable(core))
            self.assertTrue(set(result.core_sources).issubset(result.active_sources))

    def test_add_delete_reactivate_and_replace_source(self) -> None:
        session = PersistentSatModel()
        snapshot = {11: (positive(A),), 12: (negative(A), positive(B))}
        first = session.reconcile(snapshot, context="grounding-v1")
        self.assertTrue(first.rebuilt)
        selector_a = session.selector_for_source(11)
        atom_a = session.atom_variable(A)
        self.assert_agrees(session, snapshot)

        snapshot[13] = (negative(B),)
        second = session.reconcile(snapshot, context="grounding-v1")
        self.assertFalse(second.rebuilt)
        self.assertEqual(second.retained, 2)
        self.assertEqual(session.selector_for_source(11), selector_a)
        self.assertEqual(session.atom_variable(A), atom_a)
        self.assert_agrees(session, snapshot)

        del snapshot[11]
        third = session.reconcile(snapshot, context="grounding-v1")
        self.assertEqual(third.retired, 1)
        self.assertNotIn(11, session.solve().active_sources)

        snapshot[11] = (positive(A),)
        fourth = session.reconcile(snapshot, context="grounding-v1")
        self.assertEqual(fourth.reactivated, 1)
        self.assertEqual(session.selector_for_source(11), selector_a)

        snapshot[11] = (positive(C),)
        fifth = session.reconcile(snapshot, context="grounding-v1")
        self.assertEqual(fifth.added, 1)
        self.assertNotEqual(session.selector_for_source(11), selector_a)
        self.assert_agrees(session, snapshot)

    def test_core_maps_only_current_sources_across_retirement(self) -> None:
        session = PersistentSatModel()
        old = {1: (positive(C),), 2: (negative(C),)}
        session.reconcile(old, context="g")
        self.assertEqual(session.solve().core_sources, (1, 2))

        current = {
            10: (positive(A),),
            11: (negative(A),),
            12: (positive(D),),
        }
        session.reconcile(current, context="g")
        result = session.solve()
        self.assertFalse(result.satisfiable)
        self.assertEqual(result.core_sources, (10, 11))
        self.assertTrue(set(result.core_sources).isdisjoint(old))

    def test_forced_renumbering_preserves_outcome_and_source_core(self) -> None:
        session = PersistentSatModel(
            variable_cap=7,
            minimum_permanent_limit=100,
            minimum_retired_limit=100,
        )
        snapshot = {
            20: (positive(D),),
            21: (negative(D),),
            30: (positive(B),),
            31: (positive(C),),
        }
        session.reconcile(snapshot, context="g")
        first_epoch = session.epoch
        first_atom = session.atom_variable(D)
        first_core = session.solve().core_sources

        snapshot = {
            20: (positive(D),),
            21: (negative(D),),
            22: (positive(A),),
        }
        transition = session.reconcile(snapshot, context="g")
        self.assertTrue(transition.rebuilt)
        self.assertGreater(session.epoch, first_epoch)
        self.assertNotEqual(session.atom_variable(D), first_atom)
        self.assertEqual(session.solve().core_sources, first_core)
        self.assert_agrees(session, snapshot)

    def test_growth_bounds_force_compaction(self) -> None:
        session = PersistentSatModel(
            minimum_permanent_limit=4,
            permanent_factor=2,
            minimum_retired_limit=2,
            retired_factor=1,
        )
        saw_rebuild = False
        for index in range(20):
            key = ATOMS[index % len(ATOMS)]
            snapshot = {1: (positive(key),)}
            transition = session.reconcile(snapshot, context="g")
            saw_rebuild |= transition.rebuilt and index != 0
            self.assertLessEqual(
                session.permanent_clause_count, session.permanent_limit()
            )
            self.assertLessEqual(
                session.retired_clause_count, session.retired_limit()
            )
            self.assert_agrees(session, snapshot)
        self.assertTrue(saw_rebuild)

    def test_partial_add_and_reset_failures_poison_until_rebuild(self) -> None:
        session = PersistentSatModel(
            minimum_permanent_limit=100,
            minimum_retired_limit=100,
        )
        session.reconcile({1: (positive(A),)}, context="g")
        target = {
            1: (positive(A),),
            2: (negative(A), positive(B)),
            3: (negative(B),),
        }
        with self.assertRaises(InjectedBackendFailure):
            session.reconcile(
                target,
                context="g",
                failure=FailurePlan(fail_add_after=1),
            )
        with self.assertRaises(SessionPoisoned):
            session.solve()

        recovered = session.reconcile(target, context="g")
        self.assertTrue(recovered.rebuilt)
        self.assertFalse(session.poisoned)
        self.assert_agrees(session, target)

        with self.assertRaises(InjectedBackendFailure):
            session.reset(failure=FailurePlan(fail_reset=True))
        with self.assertRaises(SessionPoisoned):
            session.solve()
        session.reconcile(target, context="g")
        self.assert_agrees(session, target)

    def test_rebuild_failure_and_capacity_failure_are_fail_closed(self) -> None:
        session = PersistentSatModel()
        target = {1: (positive(A),), 2: (negative(A),)}
        with self.assertRaises(InjectedBackendFailure):
            session.reconcile(
                target,
                context="g",
                failure=FailurePlan(fail_add_after=1),
            )
        with self.assertRaises(SessionPoisoned):
            session.solve()
        session.reconcile(target, context="g")
        self.assertFalse(session.solve().satisfiable)

        too_small = PersistentSatModel(variable_cap=1)
        with self.assertRaises(CapacityError):
            too_small.reconcile(target, context="g")
        with self.assertRaises(SessionPoisoned):
            too_small.solve()

    def test_context_reset_drops_retired_database_and_changes_epoch(self) -> None:
        session = PersistentSatModel()
        session.reconcile({1: (positive(A),)}, context="grounding-a")
        session.reconcile({2: (positive(B),)}, context="grounding-a")
        self.assertGreater(session.retired_clause_count, 0)
        prior_epoch = session.epoch

        snapshot = {2: (positive(B),)}
        transition = session.reconcile(snapshot, context="grounding-b")
        self.assertTrue(transition.rebuilt)
        self.assertGreater(session.epoch, prior_epoch)
        self.assertEqual(session.retired_clause_count, 0)
        self.assertEqual(session.permanent_clause_count, 1)
        self.assert_agrees(session, snapshot)

    def test_empty_duplicates_tautology_and_contradictory_units(self) -> None:
        cases = (
            {},
            {1: (positive(A), negative(A))},
            {1: (positive(A), positive(A)), 2: (negative(A),)},
            {1: ()},
        )
        for index, snapshot in enumerate(cases):
            with self.subTest(index=index):
                session = PersistentSatModel()
                session.reconcile(snapshot, context="g")
                self.assert_agrees(session, snapshot)

    def test_structural_atom_key_is_not_a_hash_or_display_number(self) -> None:
        unary = atom("f", A)
        other_sort = atom("a", sort="$o")
        session = PersistentSatModel()
        snapshot = {
            1: (positive(A),),
            2: (positive(unary),),
            3: (positive(other_sort),),
        }
        session.reconcile(snapshot, context="g")
        self.assertEqual(len(session.atom_variables), 3)
        self.assertEqual(len(set(session.atom_variables.values())), 3)

    def test_100_randomized_transition_traces_match_fresh_oracle(self) -> None:
        result = run_campaign(seed_count=100, steps_per_trace=60)
        metrics = result["metrics"]
        self.assertEqual(result["status"], "pass")
        self.assertEqual(metrics["traces"], 100)
        self.assertEqual(metrics["steps_per_trace"], 60)
        self.assertEqual(metrics["transitions"], 6_000)
        self.assertEqual(metrics["oracle_checks"], 6_000)
        self.assertEqual(
            metrics["rebuilds"] + metrics["incremental_transitions"],
            6_000,
        )


if __name__ == "__main__":
    unittest.main()
