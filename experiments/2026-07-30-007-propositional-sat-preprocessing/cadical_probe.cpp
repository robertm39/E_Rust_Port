// Experiment-only CaDiCaL preprocessing probe.
//
// This uses only the pinned public C++ API. It is not linked into Umlaut.

#include "cadical.hpp"

#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

using Clock = std::chrono::steady_clock;

std::uint64_t elapsed_ns(const Clock::time_point start) {
  return std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() -
                                                              start)
      .count();
}

void print_model(CaDiCaL::Solver &solver, int variables) {
  std::cout << '[';
  for (int variable = 1; variable <= variables; ++variable) {
    if (variable != 1)
      std::cout << ',';
    const int value = solver.val(variable);
    std::cout << (value < 0 ? -variable : variable);
  }
  std::cout << ']';
}

int parse_limit(const char *text) {
  const long parsed = std::strtol(text, nullptr, 10);
  if (parsed < -1 || parsed > INT32_MAX)
    throw std::runtime_error("decision limit is out of range");
  return static_cast<int>(parsed);
}

} // namespace

int main(int argc, char **argv) {
  if (argc < 4 || argc > 6) {
    std::cerr << "usage: cadical-probe plain|default INPUT DECISIONS "
                 "[PROOF|- [SIMPLIFIED]]\n";
    return 2;
  }

  try {
    const std::string configuration(argv[1]);
    const std::string input(argv[2]);
    const int decision_limit = parse_limit(argv[3]);
    const char *proof_path =
        argc >= 5 && std::string(argv[4]) != "-" ? argv[4] : nullptr;
    const char *simplified_path = argc == 6 ? argv[5] : nullptr;
    if (configuration != "plain" && configuration != "default")
      throw std::runtime_error("configuration must be plain or default");

    CaDiCaL::Solver solver;
    if (!solver.configure(configuration.c_str()))
      throw std::runtime_error("CaDiCaL rejected the configuration");
    if (!solver.set("quiet", 1))
      throw std::runtime_error("CaDiCaL rejected the quiet option");
    if (proof_path != nullptr && !solver.trace_proof(proof_path))
      throw std::runtime_error("CaDiCaL could not open the proof trace");

    int declared_variables = 0;
    const auto insertion_start = Clock::now();
    const char *read_error =
        solver.read_dimacs(input.c_str(), declared_variables, 2);
    const std::uint64_t insertion_ns = elapsed_ns(insertion_start);
    if (read_error != nullptr)
      throw std::runtime_error(read_error);

    const int active_before = solver.active();
    const std::int64_t clauses_before = solver.irredundant();
    if (decision_limit >= 0)
      solver.limit("decisions", decision_limit);

    const auto simplify_start = Clock::now();
    int result = solver.simplify(3);
    const std::uint64_t simplify_ns = elapsed_ns(simplify_start);
    const int active_after = solver.active();
    const std::int64_t clauses_after = solver.irredundant();
    if (simplified_path != nullptr) {
      const char *write_error =
          solver.write_dimacs(simplified_path, declared_variables);
      if (write_error != nullptr)
        throw std::runtime_error(write_error);
    }

    std::uint64_t solve_ns = 0;
    if (result == 0) {
      if (decision_limit >= 0)
        solver.limit("decisions", decision_limit);
      const auto solve_start = Clock::now();
      result = solver.solve();
      solve_ns = elapsed_ns(solve_start);
    }

    if (result == 20 && proof_path != nullptr)
      solver.conclude();
    if (proof_path != nullptr)
      solver.close_proof_trace();

    const char *status =
        result == 10 ? "sat" : (result == 20 ? "unsat" : "unknown");
    std::cout << "{\"backend\":\"cadical-" << configuration
              << "\",\"status\":\"" << status
              << "\",\"variables\":" << declared_variables
              << ",\"active_before\":" << active_before
              << ",\"active_after\":" << active_after
              << ",\"clauses_before\":" << clauses_before
              << ",\"clauses_after\":" << clauses_after
              << ",\"insertion_ns\":" << insertion_ns
              << ",\"simplify_ns\":" << simplify_ns
              << ",\"solve_ns\":" << solve_ns << ",\"model\":";
    if (result == 10)
      print_model(solver, declared_variables);
    else
      std::cout << "[]";
    std::cout << "}\n";
    return 0;
  } catch (const std::exception &error) {
    std::cerr << "cadical-probe: " << error.what() << '\n';
    return 1;
  }
}
