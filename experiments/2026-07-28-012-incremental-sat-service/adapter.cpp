// Backend-neutral incremental SAT experiment adapter.
//
// Compile exactly one of ADAPTER_CADICAL, ADAPTER_PICOSAT, or
// ADAPTER_MINISAT. This file only calls documented public backend APIs.

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <limits>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

#if defined(ADAPTER_CADICAL)
#include "ccadical.h"
#elif defined(ADAPTER_PICOSAT)
extern "C" {
#include "picosat.h"
}
#elif defined(ADAPTER_MINISAT)
#include "minisat/core/Solver.h"
#else
#error "select one SAT backend"
#endif

namespace {

using Clock = std::chrono::steady_clock;

struct QueryResult {
  std::string status;
  std::vector<int> model;
  std::vector<int> core;
  std::uint64_t elapsed_ns = 0;
  std::uint64_t core_ns = 0;
  std::uint64_t decisions = 0;
  std::uint64_t conflicts = 0;
  std::uint64_t propagations = 0;
};

struct Deadline {
  Clock::time_point expires;
  bool enabled = false;
};

int deadline_expired(void *raw) {
  const auto *deadline = static_cast<const Deadline *>(raw);
  return deadline->enabled && Clock::now() >= deadline->expires;
}

std::string json_escape(const std::string &text) {
  std::ostringstream output;
  for (const char character : text) {
    switch (character) {
    case '"':
      output << "\\\"";
      break;
    case '\\':
      output << "\\\\";
      break;
    case '\n':
      output << "\\n";
      break;
    case '\r':
      output << "\\r";
      break;
    case '\t':
      output << "\\t";
      break;
    default:
      output << character;
      break;
    }
  }
  return output.str();
}

void write_int_array(std::ostream &output, const std::vector<int> &values) {
  output << '[';
  for (std::size_t index = 0; index < values.size(); ++index) {
    if (index != 0) {
      output << ',';
    }
    output << values[index];
  }
  output << ']';
}

class Backend {
public:
  Backend(int max_variable, const std::string &proof_path)
      : max_variable_(max_variable), proof_path_(proof_path) {}
  virtual ~Backend() = default;

  virtual void add_clause(const std::vector<int> &clause) = 0;
  virtual QueryResult solve(const std::vector<int> &assumptions, int limit,
                            std::uint64_t deadline_us) = 0;
  virtual const char *name() const = 0;
  virtual const char *version() const = 0;
  virtual const char *limit_kind() const = 0;
  virtual bool native_deadline() const = 0;
  virtual bool proof_capable() const = 0;

protected:
  int max_variable_;
  std::string proof_path_;
};

#if defined(ADAPTER_CADICAL)

class SelectedBackend final : public Backend {
public:
  SelectedBackend(int max_variable, const std::string &proof_path)
      : Backend(max_variable, proof_path), solver_(ccadical_init()) {
    if (solver_ == nullptr) {
      throw std::runtime_error("ccadical_init returned null");
    }
    ccadical_set_terminate(solver_, &deadline_, deadline_expired);
    if (!proof_path_.empty()) {
      proof_file_ = std::fopen(proof_path_.c_str(), "wb");
      if (proof_file_ == nullptr ||
          ccadical_trace_proof(solver_, proof_file_, proof_path_.c_str()) == 0) {
        throw std::runtime_error("could not enable CaDiCaL proof tracing");
      }
    }
  }

  ~SelectedBackend() override {
    if (solver_ != nullptr) {
      if (proof_file_ != nullptr) {
        ccadical_close_proof(solver_);
      }
      ccadical_release(solver_);
    }
    if (proof_file_ != nullptr) {
      std::fclose(proof_file_);
    }
  }

  void add_clause(const std::vector<int> &clause) override {
    for (const int literal : clause) {
      ccadical_add(solver_, literal);
    }
    ccadical_add(solver_, 0);
  }

  QueryResult solve(const std::vector<int> &assumptions, int limit,
                    std::uint64_t deadline_us) override {
    if (limit >= 0) {
      ccadical_limit(solver_, "decisions", limit);
    }
    deadline_.enabled = deadline_us != 0;
    if (deadline_.enabled) {
      deadline_.expires = Clock::now() + std::chrono::microseconds(deadline_us);
    }
    for (const int literal : assumptions) {
      ccadical_assume(solver_, literal);
    }
    const auto started = Clock::now();
    const int raw_result = ccadical_solve(solver_);
    const auto finished = Clock::now();
    deadline_.enabled = false;

    QueryResult result;
    result.elapsed_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(
                            finished - started)
                            .count();
    if (raw_result == 10) {
      result.status = "sat";
      for (int variable = 1; variable <= max_variable_; ++variable) {
        const int value = ccadical_val(solver_, variable);
        result.model.push_back(value < 0 ? -variable : variable);
      }
    } else if (raw_result == 20) {
      result.status = "unsat";
      const auto core_started = Clock::now();
      for (const int literal : assumptions) {
        if (ccadical_failed(solver_, literal) != 0) {
          result.core.push_back(literal);
        }
      }
      result.core_ns =
          std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() -
                                                               core_started)
              .count();
      if (!proof_path_.empty()) {
        ccadical_conclude(solver_);
      }
    } else {
      result.status = "unknown";
    }
    return result;
  }

  const char *name() const override { return "cadical"; }
  const char *version() const override { return ccadical_signature(); }
  const char *limit_kind() const override { return "decisions"; }
  bool native_deadline() const override { return true; }
  bool proof_capable() const override { return true; }

private:
  CCaDiCaL *solver_;
  FILE *proof_file_ = nullptr;
  Deadline deadline_;
};

#elif defined(ADAPTER_PICOSAT)

class SelectedBackend final : public Backend {
public:
  SelectedBackend(int max_variable, const std::string &proof_path)
      : Backend(max_variable, proof_path), solver_(picosat_init()) {
    if (solver_ == nullptr) {
      throw std::runtime_error("picosat_init returned null");
    }
    picosat_set_interrupt(solver_, &deadline_, deadline_expired);
    if (!proof_path_.empty()) {
      picosat_enable_trace_generation(solver_);
    }
  }

  ~SelectedBackend() override {
    if (solver_ != nullptr) {
      picosat_reset(solver_);
    }
  }

  void add_clause(const std::vector<int> &clause) override {
    for (const int literal : clause) {
      picosat_add(solver_, literal);
    }
    picosat_add(solver_, 0);
  }

  QueryResult solve(const std::vector<int> &assumptions, int limit,
                    std::uint64_t deadline_us) override {
    deadline_.enabled = deadline_us != 0;
    if (deadline_.enabled) {
      deadline_.expires = Clock::now() + std::chrono::microseconds(deadline_us);
    }
    for (const int literal : assumptions) {
      picosat_assume(solver_, literal);
    }
    const auto started = Clock::now();
    const int raw_result = picosat_sat(solver_, limit);
    const auto finished = Clock::now();
    deadline_.enabled = false;

    QueryResult result;
    result.elapsed_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(
                            finished - started)
                            .count();
    if (raw_result == 10) {
      result.status = "sat";
      for (int variable = 1; variable <= max_variable_; ++variable) {
        const int value = picosat_deref(solver_, variable);
        result.model.push_back(value < 0 ? -variable : variable);
      }
    } else if (raw_result == 20) {
      result.status = "unsat";
      const auto core_started = Clock::now();
      for (const int literal : assumptions) {
        if (picosat_failed_assumption(solver_, literal) != 0) {
          result.core.push_back(literal);
        }
      }
      result.core_ns =
          std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() -
                                                               core_started)
              .count();
      if (!proof_path_.empty()) {
        FILE *proof = std::fopen(proof_path_.c_str(), "wb");
        if (proof == nullptr) {
          throw std::runtime_error("could not open PicoSAT proof path");
        }
        picosat_write_rup_trace(solver_, proof);
        std::fclose(proof);
      }
    } else {
      result.status = "unknown";
    }
    return result;
  }

  const char *name() const override { return "picosat"; }
  const char *version() const override { return picosat_version(); }
  const char *limit_kind() const override { return "decisions"; }
  bool native_deadline() const override { return true; }
  bool proof_capable() const override { return true; }

private:
  PicoSAT *solver_;
  Deadline deadline_;
};

#elif defined(ADAPTER_MINISAT)

class SelectedBackend final : public Backend {
public:
  SelectedBackend(int max_variable, const std::string &proof_path)
      : Backend(max_variable, proof_path) {
    while (solver_.nVars() < max_variable_) {
      solver_.newVar();
    }
  }

  void add_clause(const std::vector<int> &clause) override {
    Minisat::vec<Minisat::Lit> converted;
    for (const int literal : clause) {
      ensure_variable(std::abs(literal));
      converted.push(to_literal(literal));
    }
    solver_.addClause(converted);
  }

  QueryResult solve(const std::vector<int> &assumptions, int limit,
                    std::uint64_t deadline_us) override {
    Minisat::vec<Minisat::Lit> converted;
    for (const int literal : assumptions) {
      ensure_variable(std::abs(literal));
      converted.push(to_literal(literal));
    }
    solver_.budgetOff();
    if (limit >= 0) {
      solver_.setConfBudget(limit);
    }
    solver_.clearInterrupt();
    std::atomic<bool> finished{false};
    std::thread interrupter;
    if (deadline_us != 0) {
      interrupter = std::thread([this, deadline_us, &finished]() {
        std::this_thread::sleep_for(std::chrono::microseconds(deadline_us));
        if (!finished.load(std::memory_order_acquire)) {
          solver_.interrupt();
        }
      });
    }

    const auto before_decisions = solver_.decisions;
    const auto before_conflicts = solver_.conflicts;
    const auto before_propagations = solver_.propagations;
    const auto started = Clock::now();
    const Minisat::lbool raw_result = solver_.solveLimited(converted);
    const auto ended = Clock::now();
    finished.store(true, std::memory_order_release);
    if (interrupter.joinable()) {
      interrupter.join();
    }

    QueryResult result;
    result.elapsed_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(
                            ended - started)
                            .count();
    result.decisions = solver_.decisions - before_decisions;
    result.conflicts = solver_.conflicts - before_conflicts;
    result.propagations = solver_.propagations - before_propagations;
    if (raw_result == Minisat::l_True) {
      result.status = "sat";
      for (int variable = 1; variable <= max_variable_; ++variable) {
        const Minisat::lbool value = solver_.modelValue(variable - 1);
        result.model.push_back(value == Minisat::l_False ? -variable : variable);
      }
    } else if (raw_result == Minisat::l_False) {
      result.status = "unsat";
      const auto core_started = Clock::now();
      for (const int literal : assumptions) {
        if (conflict_contains(~to_literal(literal))) {
          result.core.push_back(literal);
        }
      }
      result.core_ns =
          std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now() -
                                                               core_started)
              .count();
    } else {
      result.status = "unknown";
    }
    return result;
  }

  const char *name() const override { return "minisat"; }
  const char *version() const override { return "2.2.0-37dc6c6"; }
  const char *limit_kind() const override { return "conflicts"; }
  bool native_deadline() const override { return true; }
  bool proof_capable() const override { return false; }

private:
  static Minisat::Lit to_literal(int literal) {
    return Minisat::mkLit(std::abs(literal) - 1, literal < 0);
  }

  void ensure_variable(int variable) {
    while (solver_.nVars() < variable) {
      solver_.newVar();
    }
    max_variable_ = std::max(max_variable_, variable);
  }

  bool conflict_contains(Minisat::Lit target) const {
    for (int index = 0; index < solver_.conflict.size(); ++index) {
      if (solver_.conflict[index] == target) {
        return true;
      }
    }
    return false;
  }

  Minisat::Solver solver_;
};

#endif

std::vector<int> parse_literals(std::istringstream &line) {
  std::vector<int> literals;
  int literal = 0;
  bool terminated = false;
  while (line >> literal) {
    if (literal == 0) {
      terminated = true;
      break;
    }
    literals.push_back(literal);
  }
  if (!terminated) {
    throw std::runtime_error("literal list is not zero-terminated");
  }
  return literals;
}

void emit_result(const Backend &backend, const std::string &session,
                 const std::string &query_id, std::size_t clause_count,
                 const std::vector<int> &assumptions,
                 const QueryResult &result, std::uint64_t insertion_ns) {
  std::cout << "{\"backend\":\"" << json_escape(backend.name())
            << "\",\"version\":\"" << json_escape(backend.version())
            << "\",\"session\":\"" << json_escape(session)
            << "\",\"query\":\"" << json_escape(query_id)
            << "\",\"clauses\":" << clause_count
            << ",\"assumptions\":" << assumptions.size()
            << ",\"status\":\"" << result.status << "\",\"elapsed_ns\":"
            << result.elapsed_ns << ",\"core_ns\":" << result.core_ns
            << ",\"insertion_ns\":" << insertion_ns
            << ",\"native_limit_kind\":\""
            << backend.limit_kind() << "\",\"native_deadline\":"
            << (backend.native_deadline() ? "true" : "false")
            << ",\"proof_capable\":"
            << (backend.proof_capable() ? "true" : "false")
            << ",\"decisions\":" << result.decisions
            << ",\"conflicts\":" << result.conflicts
            << ",\"propagations\":" << result.propagations << ",\"model\":";
  write_int_array(std::cout, result.model);
  std::cout << ",\"core\":";
  write_int_array(std::cout, result.core);
  std::cout << "}\n";
}

int run(const std::string &path, const std::string &proof_path) {
  std::ifstream input(path);
  if (!input) {
    throw std::runtime_error("could not open session: " + path);
  }

  std::string line;
  std::size_t line_number = 0;
  int max_variable = 0;
  bool saw_header = false;
  std::unique_ptr<Backend> backend;
  std::size_t clause_count = 0;
  std::uint64_t insertion_ns = 0;
  while (std::getline(input, line)) {
    ++line_number;
    if (line.empty() || line[0] == 'c') {
      continue;
    }
    std::istringstream fields(line);
    std::string opcode;
    fields >> opcode;
    if (opcode == "p") {
      std::string format;
      fields >> format >> max_variable;
      if (format != "isat" || max_variable < 0 || backend) {
        throw std::runtime_error("invalid or duplicate session header");
      }
      backend = std::make_unique<SelectedBackend>(max_variable, proof_path);
      saw_header = true;
    } else if (opcode == "a") {
      if (!backend) {
        throw std::runtime_error("clause precedes session header");
      }
      const auto clause = parse_literals(fields);
      const auto insertion_started = Clock::now();
      backend->add_clause(clause);
      insertion_ns +=
          std::chrono::duration_cast<std::chrono::nanoseconds>(
              Clock::now() - insertion_started)
              .count();
      ++clause_count;
    } else if (opcode == "q") {
      if (!backend) {
        throw std::runtime_error("query precedes session header");
      }
      std::string query_id;
      int limit = -1;
      std::uint64_t deadline_us = 0;
      if (!(fields >> query_id >> limit >> deadline_us)) {
        throw std::runtime_error("invalid query prefix");
      }
      const auto assumptions = parse_literals(fields);
      const auto result = backend->solve(assumptions, limit, deadline_us);
      emit_result(*backend, path, query_id, clause_count, assumptions, result,
                  insertion_ns);
    } else {
      throw std::runtime_error("unknown opcode at line " +
                               std::to_string(line_number));
    }
  }
  if (!saw_header) {
    throw std::runtime_error("session has no header");
  }
  return 0;
}

} // namespace

int main(int argc, char **argv) {
  try {
    if (argc != 2 && argc != 3) {
      std::cerr << "usage: sat-adapter SESSION [PROOF]\n";
      return 2;
    }
    return run(argv[1], argc == 3 ? argv[2] : "");
  } catch (const std::exception &error) {
    std::cerr << "sat-adapter: " << error.what() << '\n';
    return 1;
  }
}
