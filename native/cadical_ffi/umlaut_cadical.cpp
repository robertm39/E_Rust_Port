#include "umlaut_cadical.h"

#include "cadical.hpp"

#include <cstdio>
#include <cstring>
#include <exception>
#include <new>

namespace {

constexpr std::size_t ERROR_CAPACITY = 512;

class TerminationProxy final : public CaDiCaL::Terminator {
public:
  bool terminate() noexcept override {
    return callback_ != nullptr && callback_(state_) != 0;
  }

  void set(void *state, UmlautCadicalTerminate callback) noexcept {
    state_ = state;
    callback_ = callback;
  }

private:
  void *state_ = nullptr;
  UmlautCadicalTerminate callback_ = nullptr;
};

} // namespace

struct UmlautCadical {
  CaDiCaL::Solver solver;
  TerminationProxy termination;
  char error[ERROR_CAPACITY] = {};
  bool proof_open = false;

  UmlautCadical() { solver.connect_terminator(&termination); }
};

namespace {

void clear_error(UmlautCadical *wrapper) noexcept {
  if (wrapper != nullptr)
    wrapper->error[0] = '\0';
}

void set_error(UmlautCadical *wrapper, const char *message) noexcept {
  if (wrapper == nullptr)
    return;
  if (message == nullptr)
    message = "unknown C++ exception";
  std::snprintf(wrapper->error, ERROR_CAPACITY, "%s", message);
}

template <typename Action>
int call(UmlautCadical *wrapper, Action action) noexcept {
  if (wrapper == nullptr)
    return 0;
  clear_error(wrapper);
  try {
    action();
    return 1;
  } catch (const std::exception &exception) {
    set_error(wrapper, exception.what());
  } catch (...) {
    set_error(wrapper, "unknown C++ exception");
  }
  return 0;
}

} // namespace

extern "C" {

const char *umlaut_cadical_signature(void) {
  try {
    return CaDiCaL::Solver::signature();
  } catch (...) {
    return nullptr;
  }
}

UmlautCadical *umlaut_cadical_init(void) {
  try {
    return new UmlautCadical();
  } catch (...) {
    return nullptr;
  }
}

void umlaut_cadical_release(UmlautCadical *wrapper) {
  try {
    delete wrapper;
  } catch (...) {
  }
}

const char *umlaut_cadical_last_error(const UmlautCadical *wrapper) {
  if (wrapper == nullptr)
    return "null CaDiCaL wrapper";
  return wrapper->error;
}

int umlaut_cadical_set_terminate(
    UmlautCadical *wrapper,
    void *state,
    UmlautCadicalTerminate callback
) {
  return call(wrapper, [=]() { wrapper->termination.set(state, callback); });
}

int umlaut_cadical_add(UmlautCadical *wrapper, int literal) {
  return call(wrapper, [=]() { wrapper->solver.add(literal); });
}

int umlaut_cadical_assume(UmlautCadical *wrapper, int literal) {
  return call(wrapper, [=]() { wrapper->solver.assume(literal); });
}

int umlaut_cadical_limit_decisions(UmlautCadical *wrapper, int limit) {
  if (wrapper == nullptr)
    return 0;
  clear_error(wrapper);
  try {
    if (wrapper->solver.limit("decisions", limit))
      return 1;
    set_error(wrapper, "CaDiCaL rejected the decisions limit");
  } catch (const std::exception &exception) {
    set_error(wrapper, exception.what());
  } catch (...) {
    set_error(wrapper, "unknown C++ exception");
  }
  return 0;
}

int umlaut_cadical_solve(UmlautCadical *wrapper) {
  if (wrapper == nullptr)
    return -1;
  clear_error(wrapper);
  try {
    return wrapper->solver.solve();
  } catch (const std::exception &exception) {
    set_error(wrapper, exception.what());
  } catch (...) {
    set_error(wrapper, "unknown C++ exception");
  }
  return -1;
}

int umlaut_cadical_val(UmlautCadical *wrapper, int literal) {
  if (wrapper == nullptr)
    return 0;
  clear_error(wrapper);
  try {
    return wrapper->solver.val(literal);
  } catch (const std::exception &exception) {
    set_error(wrapper, exception.what());
  } catch (...) {
    set_error(wrapper, "unknown C++ exception");
  }
  return 0;
}

int umlaut_cadical_failed(UmlautCadical *wrapper, int literal) {
  if (wrapper == nullptr)
    return -1;
  clear_error(wrapper);
  try {
    return wrapper->solver.failed(literal) ? 1 : 0;
  } catch (const std::exception &exception) {
    set_error(wrapper, exception.what());
  } catch (...) {
    set_error(wrapper, "unknown C++ exception");
  }
  return -1;
}

int umlaut_cadical_trace_proof(UmlautCadical *wrapper, const char *path) {
  if (wrapper == nullptr || path == nullptr)
    return 0;
  clear_error(wrapper);
  try {
    if (!wrapper->solver.trace_proof(path)) {
      set_error(wrapper, "CaDiCaL could not open the proof trace");
      return 0;
    }
    wrapper->proof_open = true;
    return 1;
  } catch (const std::exception &exception) {
    set_error(wrapper, exception.what());
  } catch (...) {
    set_error(wrapper, "unknown C++ exception");
  }
  return 0;
}

int umlaut_cadical_conclude(UmlautCadical *wrapper) {
  return call(wrapper, [=]() { wrapper->solver.conclude(); });
}

int umlaut_cadical_close_proof(UmlautCadical *wrapper) {
  return call(wrapper, [=]() {
    if (wrapper->proof_open) {
      wrapper->solver.close_proof_trace();
      wrapper->proof_open = false;
    }
  });
}

} // extern "C"
