// Experiment-only CaDiCaL solve/model/proof adapter.
//
// This program uses only the pinned public CaDiCaL C++ API and is not linked
// into Umlaut.

#include "cadical.hpp"

#include <chrono>
#include <cstdint>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>

namespace {

using Clock = std::chrono::steady_clock;

std::uint64_t elapsed_ns(const Clock::time_point start) {
  return std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() -
                                                              start)
      .count();
}

void print_model(CaDiCaL::Solver &solver, const int variables) {
  std::cout << '[';
  for (int variable = 1; variable <= variables; ++variable) {
    if (variable != 1)
      std::cout << ',';
    const int value = solver.val(variable);
    std::cout << (value < 0 ? -variable : variable);
  }
  std::cout << ']';
}

} // namespace

int main(int argc, char **argv) {
  if (argc < 2 || argc > 3) {
    std::cerr << "usage: cadical-driver INPUT [PROOF]\n";
    return 2;
  }
  try {
    CaDiCaL::Solver solver;
    if (!solver.configure("plain"))
      throw std::runtime_error("CaDiCaL rejected the plain configuration");
    if (!solver.set("quiet", 1))
      throw std::runtime_error("CaDiCaL rejected the quiet option");
    const char *proof_path = argc == 3 ? argv[2] : nullptr;
    if (proof_path != nullptr && !solver.trace_proof(proof_path))
      throw std::runtime_error("CaDiCaL could not open the proof trace");

    int variables = 0;
    const auto read_start = Clock::now();
    const char *read_error = solver.read_dimacs(argv[1], variables, 2);
    const std::uint64_t read_ns = elapsed_ns(read_start);
    if (read_error != nullptr)
      throw std::runtime_error(read_error);

    const auto solve_start = Clock::now();
    const int result = solver.solve();
    const std::uint64_t solve_ns = elapsed_ns(solve_start);
    if (result == 20 && proof_path != nullptr)
      solver.conclude();
    if (proof_path != nullptr)
      solver.close_proof_trace();

    const char *status =
        result == 10 ? "sat" : (result == 20 ? "unsat" : "unknown");
    std::cout << "{\"status\":\"" << status << "\",\"variables\":"
              << variables << ",\"clauses\":" << solver.irredundant()
              << ",\"read_ns\":" << read_ns << ",\"solve_ns\":" << solve_ns
              << ",\"model\":";
    if (result == 10)
      print_model(solver, variables);
    else
      std::cout << "[]";
    std::cout << "}\n";
    return 0;
  } catch (const std::exception &error) {
    std::cerr << "cadical-driver: " << error.what() << '\n';
    return 1;
  }
}
