//! Experiment-only incremental Rust binding to the pinned Z3 C API.
//!
//! This driver deliberately has no Cargo dependency. Its narrow unsafe boundary
//! is part of the packaging and FFI-safety evaluation, not production code.

use std::collections::HashSet;
use std::env;
use std::ffi::{c_char, c_uint, c_void, CStr, CString};
use std::fs;
use std::io;
use std::path::Path;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

type Z3Config = *mut c_void;
type Z3Context = *mut c_void;
type Z3Solver = *mut c_void;
type Z3Params = *mut c_void;
type Z3Symbol = *mut c_void;
type Z3Sort = *mut c_void;
type Z3Ast = *mut c_void;
type Z3AstVector = *mut c_void;
type Z3Model = *mut c_void;

const Z3_L_FALSE: i32 = -1;
const Z3_L_UNDEF: i32 = 0;
const Z3_L_TRUE: i32 = 1;

#[link(name = "z3")]
unsafe extern "C" {
    fn Z3_mk_config() -> Z3Config;
    fn Z3_del_config(config: Z3Config);
    fn Z3_set_param_value(config: Z3Config, key: *const c_char, value: *const c_char);
    fn Z3_mk_context_rc(config: Z3Config) -> Z3Context;
    fn Z3_del_context(context: Z3Context);
    fn Z3_get_full_version() -> *const c_char;

    fn Z3_mk_solver(context: Z3Context) -> Z3Solver;
    fn Z3_solver_inc_ref(context: Z3Context, solver: Z3Solver);
    fn Z3_solver_dec_ref(context: Z3Context, solver: Z3Solver);
    fn Z3_solver_from_string(context: Z3Context, solver: Z3Solver, text: *const c_char);
    fn Z3_solver_push(context: Z3Context, solver: Z3Solver);
    fn Z3_solver_pop(context: Z3Context, solver: Z3Solver, scopes: c_uint);
    fn Z3_solver_check(context: Z3Context, solver: Z3Solver) -> i32;
    fn Z3_solver_check_assumptions(
        context: Z3Context,
        solver: Z3Solver,
        count: c_uint,
        assumptions: *const Z3Ast,
    ) -> i32;
    fn Z3_solver_get_reason_unknown(context: Z3Context, solver: Z3Solver) -> *const c_char;
    fn Z3_solver_get_unsat_core(context: Z3Context, solver: Z3Solver) -> Z3AstVector;
    fn Z3_solver_get_model(context: Z3Context, solver: Z3Solver) -> Z3Model;
    fn Z3_solver_set_params(context: Z3Context, solver: Z3Solver, params: Z3Params);

    fn Z3_mk_params(context: Z3Context) -> Z3Params;
    fn Z3_params_inc_ref(context: Z3Context, params: Z3Params);
    fn Z3_params_dec_ref(context: Z3Context, params: Z3Params);
    fn Z3_params_set_bool(context: Z3Context, params: Z3Params, key: Z3Symbol, value: i32);
    fn Z3_params_set_uint(context: Z3Context, params: Z3Params, key: Z3Symbol, value: c_uint);

    fn Z3_ast_vector_inc_ref(context: Z3Context, vector: Z3AstVector);
    fn Z3_ast_vector_dec_ref(context: Z3Context, vector: Z3AstVector);
    fn Z3_ast_vector_size(context: Z3Context, vector: Z3AstVector) -> c_uint;
    fn Z3_ast_vector_get(context: Z3Context, vector: Z3AstVector, index: c_uint) -> Z3Ast;

    fn Z3_model_inc_ref(context: Z3Context, model: Z3Model);
    fn Z3_model_dec_ref(context: Z3Context, model: Z3Model);
    fn Z3_model_eval(
        context: Z3Context,
        model: Z3Model,
        expression: Z3Ast,
        completion: i32,
        value: *mut Z3Ast,
    ) -> i32;

    fn Z3_mk_string_symbol(context: Z3Context, name: *const c_char) -> Z3Symbol;
    fn Z3_mk_bool_sort(context: Z3Context) -> Z3Sort;
    fn Z3_mk_int_sort(context: Z3Context) -> Z3Sort;
    fn Z3_mk_real_sort(context: Z3Context) -> Z3Sort;
    fn Z3_mk_const(context: Z3Context, symbol: Z3Symbol, sort: Z3Sort) -> Z3Ast;
    fn Z3_ast_to_string(context: Z3Context, ast: Z3Ast) -> *const c_char;
    fn Z3_interrupt(context: Z3Context);
}

#[derive(Debug)]
struct Assertion {
    label: String,
    expression: String,
}

#[derive(Debug)]
struct Branch {
    id: String,
    assertions: Vec<Assertion>,
}

#[derive(Debug)]
struct Workload {
    id: String,
    sort: String,
    variables: Vec<String>,
    base: Vec<Assertion>,
    branches: Vec<Branch>,
}

struct Context {
    raw: Z3Context,
}

impl Context {
    fn new() -> Result<Self, String> {
        // SAFETY: Z3 owns the returned config until Z3_del_config. All strings
        // are live NUL-terminated CStrings for the duration of each call.
        unsafe {
            let config = Z3_mk_config();
            if config.is_null() {
                return Err("Z3_mk_config returned null".to_owned());
            }
            let model_key = c_string("model")?;
            let true_value = c_string("true")?;
            Z3_set_param_value(config, model_key.as_ptr(), true_value.as_ptr());
            let raw = Z3_mk_context_rc(config);
            Z3_del_config(config);
            if raw.is_null() {
                return Err("Z3_mk_context_rc returned null".to_owned());
            }
            Ok(Self { raw })
        }
    }

    fn solver(&self) -> Result<Solver<'_>, String> {
        // SAFETY: self.raw is a live reference-counted context. The solver is
        // retained immediately and released before the context is dropped.
        unsafe {
            let raw = Z3_mk_solver(self.raw);
            if raw.is_null() {
                return Err("Z3_mk_solver returned null".to_owned());
            }
            Z3_solver_inc_ref(self.raw, raw);
            let solver = Solver { context: self, raw };
            solver.configure()?;
            Ok(solver)
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: raw was created by Z3_mk_context_rc and all Solver values
        // borrow this Context, so none can outlive this drop.
        unsafe {
            Z3_del_context(self.raw);
        }
    }
}

struct Solver<'a> {
    context: &'a Context,
    raw: Z3Solver,
}

impl Solver<'_> {
    fn configure(&self) -> Result<(), String> {
        // SAFETY: context and solver are live. params is retained for the
        // mutation and set call, then released exactly once.
        unsafe {
            let params = Z3_mk_params(self.context.raw);
            if params.is_null() {
                return Err("Z3_mk_params returned null".to_owned());
            }
            Z3_params_inc_ref(self.context.raw, params);
            let unsat_core = symbol(self.context.raw, "unsat_core")?;
            let timeout = symbol(self.context.raw, "timeout")?;
            let random_seed = symbol(self.context.raw, "random_seed")?;
            Z3_params_set_bool(self.context.raw, params, unsat_core, 1);
            Z3_params_set_uint(self.context.raw, params, timeout, 5_000);
            Z3_params_set_uint(self.context.raw, params, random_seed, 0);
            Z3_solver_set_params(self.context.raw, self.raw, params);
            Z3_params_dec_ref(self.context.raw, params);
        }
        Ok(())
    }

    fn add_script(&self, script: &str) -> Result<(), String> {
        let script = c_string(script)?;
        // SAFETY: context and solver are live, and script remains allocated
        // for the complete Z3_solver_from_string call.
        unsafe {
            Z3_solver_from_string(self.context.raw, self.raw, script.as_ptr());
        }
        Ok(())
    }

    fn push(&self) {
        // SAFETY: context and solver are live.
        unsafe {
            Z3_solver_push(self.context.raw, self.raw);
        }
    }

    fn pop(&self) {
        // SAFETY: every call follows one successful push in the same loop.
        unsafe {
            Z3_solver_pop(self.context.raw, self.raw, 1);
        }
    }

    fn check(&self) -> i32 {
        // SAFETY: context and solver are live and not accessed concurrently.
        unsafe { Z3_solver_check(self.context.raw, self.raw) }
    }

    fn check_assumptions<'a>(&self, labels: impl Iterator<Item = &'a str>) -> Result<i32, String> {
        // SAFETY: the Boolean sort, symbols, constants, solver, and assumption
        // slice all belong to the same live context and remain valid for the
        // complete Z3_solver_check_assumptions call.
        unsafe {
            let bool_sort = Z3_mk_bool_sort(self.context.raw);
            if bool_sort.is_null() {
                return Err("Z3_mk_bool_sort returned null".to_owned());
            }
            let assumptions = labels
                .map(|label| {
                    let label_symbol = symbol(self.context.raw, label)?;
                    let tracker = Z3_mk_const(self.context.raw, label_symbol, bool_sort);
                    if tracker.is_null() {
                        return Err("Z3_mk_const returned null".to_owned());
                    }
                    Ok(tracker)
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(Z3_solver_check_assumptions(
                self.context.raw,
                self.raw,
                assumptions.len() as c_uint,
                assumptions.as_ptr(),
            ))
        }
    }

    fn reason_unknown(&self) -> String {
        // SAFETY: returned text is owned by the live Z3 context.
        unsafe { c_text(Z3_solver_get_reason_unknown(self.context.raw, self.raw)) }
    }

    fn core(&self) -> Result<Vec<String>, String> {
        // SAFETY: vector belongs to the live context. It is retained while
        // elements are read, and every index is below the reported size.
        unsafe {
            let vector = Z3_solver_get_unsat_core(self.context.raw, self.raw);
            if vector.is_null() {
                return Err("Z3_solver_get_unsat_core returned null".to_owned());
            }
            Z3_ast_vector_inc_ref(self.context.raw, vector);
            let size = Z3_ast_vector_size(self.context.raw, vector);
            let mut core = Vec::with_capacity(size as usize);
            for index in 0..size {
                let ast = Z3_ast_vector_get(self.context.raw, vector, index);
                if ast.is_null() {
                    Z3_ast_vector_dec_ref(self.context.raw, vector);
                    return Err("Z3_ast_vector_get returned null".to_owned());
                }
                core.push(c_text(Z3_ast_to_string(self.context.raw, ast)));
            }
            Z3_ast_vector_dec_ref(self.context.raw, vector);
            Ok(core)
        }
    }

    fn model(&self, variables: &[String], sort_name: &str) -> Result<String, String> {
        // SAFETY: model is retained during evaluation. Created symbols, sorts,
        // constants, and values all belong to the same live context.
        unsafe {
            let model = Z3_solver_get_model(self.context.raw, self.raw);
            if model.is_null() {
                return Err("Z3_solver_get_model returned null".to_owned());
            }
            Z3_model_inc_ref(self.context.raw, model);
            let sort = match sort_name {
                "Int" => Z3_mk_int_sort(self.context.raw),
                "Real" => Z3_mk_real_sort(self.context.raw),
                _ => {
                    Z3_model_dec_ref(self.context.raw, model);
                    return Err(format!("unsupported sort: {sort_name}"));
                }
            };
            if sort.is_null() {
                Z3_model_dec_ref(self.context.raw, model);
                return Err("Z3 sort constructor returned null".to_owned());
            }
            let mut fields = Vec::with_capacity(variables.len());
            for variable in variables {
                let symbol = symbol(self.context.raw, variable)?;
                let expression = Z3_mk_const(self.context.raw, symbol, sort);
                if expression.is_null() {
                    Z3_model_dec_ref(self.context.raw, model);
                    return Err("Z3_mk_const returned null".to_owned());
                }
                let mut value = ptr::null_mut();
                if Z3_model_eval(self.context.raw, model, expression, 1, &mut value) == 0
                    || value.is_null()
                {
                    Z3_model_dec_ref(self.context.raw, model);
                    return Err(format!("could not evaluate model variable {variable}"));
                }
                fields.push(format!(
                    "{variable}={}",
                    c_text(Z3_ast_to_string(self.context.raw, value))
                ));
            }
            Z3_model_dec_ref(self.context.raw, model);
            Ok(fields.join(";"))
        }
    }
}

impl Drop for Solver<'_> {
    fn drop(&mut self) {
        // SAFETY: solver was retained once in Context::solver and the context
        // outlives this borrowed Solver.
        unsafe {
            Z3_solver_dec_ref(self.context.raw, self.raw);
        }
    }
}

fn c_string(text: &str) -> Result<CString, String> {
    CString::new(text).map_err(|_| "text contains an interior NUL".to_owned())
}

fn c_text(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return String::new();
    }
    // SAFETY: every caller passes a Z3-owned NUL-terminated string pointer
    // while the originating context remains live.
    unsafe { CStr::from_ptr(pointer).to_string_lossy().into_owned() }
}

fn symbol(context: Z3Context, text: &str) -> Result<Z3Symbol, String> {
    let text = c_string(text)?;
    // SAFETY: context is live and text remains allocated for the call.
    let result = unsafe { Z3_mk_string_symbol(context, text.as_ptr()) };
    if result.is_null() {
        return Err("Z3_mk_string_symbol returned null".to_owned());
    }
    Ok(result)
}

fn parse_protocol(path: &Path) -> Result<Vec<Workload>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut lines = text.lines();
    if lines.next() != Some("UMLAUT_GROUND_THEORY_FFI_V1") {
        return Err("invalid protocol header".to_owned());
    }
    let mut workloads = Vec::new();
    let mut current_workload: Option<Workload> = None;
    let mut current_branch: Option<Branch> = None;
    let mut ended = false;

    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let fields: Vec<&str> = line.splitn(3, '\t').collect();
        match fields.as_slice() {
            ["WORKLOAD", id, sort] if current_workload.is_none() && !ended => {
                require_symbol(id, line_number)?;
                if *sort != "Int" && *sort != "Real" {
                    return Err(format!("line {line_number}: invalid sort"));
                }
                current_workload = Some(Workload {
                    id: (*id).to_owned(),
                    sort: (*sort).to_owned(),
                    variables: Vec::new(),
                    base: Vec::new(),
                    branches: Vec::new(),
                });
            }
            ["VAR", name] if current_branch.is_none() => {
                require_symbol(name, line_number)?;
                workload_mut(&mut current_workload, line_number)?
                    .variables
                    .push((*name).to_owned());
            }
            ["BASE", label, expression] if current_branch.is_none() => {
                require_symbol(label, line_number)?;
                workload_mut(&mut current_workload, line_number)?
                    .base
                    .push(Assertion {
                        label: (*label).to_owned(),
                        expression: (*expression).to_owned(),
                    });
            }
            ["BRANCH", id] if current_branch.is_none() => {
                require_symbol(id, line_number)?;
                if current_workload.is_none() {
                    return Err(format!("line {line_number}: branch outside workload"));
                }
                current_branch = Some(Branch {
                    id: (*id).to_owned(),
                    assertions: Vec::new(),
                });
            }
            ["ASSERT", label, expression] if current_branch.is_some() => {
                require_symbol(label, line_number)?;
                current_branch
                    .as_mut()
                    .expect("guarded branch")
                    .assertions
                    .push(Assertion {
                        label: (*label).to_owned(),
                        expression: (*expression).to_owned(),
                    });
            }
            ["END_BRANCH"] => {
                let branch = current_branch
                    .take()
                    .ok_or_else(|| format!("line {line_number}: no active branch"))?;
                workload_mut(&mut current_workload, line_number)?
                    .branches
                    .push(branch);
            }
            ["END_WORKLOAD"] if current_branch.is_none() => {
                let workload = current_workload
                    .take()
                    .ok_or_else(|| format!("line {line_number}: no active workload"))?;
                validate_workload(&workload)?;
                workloads.push(workload);
            }
            ["END"] if current_workload.is_none() && current_branch.is_none() => {
                ended = true;
            }
            _ => return Err(format!("line {line_number}: malformed protocol record")),
        }
    }
    if !ended || current_workload.is_some() || current_branch.is_some() {
        return Err("incomplete protocol".to_owned());
    }
    Ok(workloads)
}

fn workload_mut(
    workload: &mut Option<Workload>,
    line_number: usize,
) -> Result<&mut Workload, String> {
    workload
        .as_mut()
        .ok_or_else(|| format!("line {line_number}: record outside workload"))
}

fn require_symbol(text: &str, line_number: usize) -> Result<(), String> {
    let valid = !text.is_empty()
        && text.bytes().enumerate().all(|(index, byte)| {
            byte == b'_'
                || byte.is_ascii_alphabetic()
                || (index > 0 && byte.is_ascii_digit())
                || (index > 0 && byte == b'-')
        });
    if valid {
        Ok(())
    } else {
        Err(format!("line {line_number}: invalid symbol {text:?}"))
    }
}

fn validate_workload(workload: &Workload) -> Result<(), String> {
    if workload.variables.is_empty() {
        return Err(format!("{} has no variables", workload.id));
    }
    if workload.branches.is_empty() {
        return Err(format!("{} has no branches", workload.id));
    }
    let mut labels = HashSet::new();
    for assertion in &workload.base {
        if !labels.insert(&assertion.label) {
            return Err(format!("{} has duplicate base label", workload.id));
        }
    }
    for branch in &workload.branches {
        let mut branch_labels = labels.clone();
        for assertion in &branch.assertions {
            if !branch_labels.insert(&assertion.label) {
                return Err(format!(
                    "{} branch {} has a duplicate label",
                    workload.id, branch.id
                ));
            }
        }
    }
    Ok(())
}

fn declarations(workload: &Workload) -> String {
    workload
        .variables
        .iter()
        .map(|variable| format!("(declare-const {variable} {})\n", workload.sort))
        .collect()
}

fn assertion_script(assertions: &[Assertion]) -> String {
    assertions
        .iter()
        .map(|assertion| {
            format!(
                "(declare-const {} Bool)\n(assert (=> {} {}))\n",
                assertion.label, assertion.label, assertion.expression
            )
        })
        .collect()
}

fn sanitize_field(text: &str) -> String {
    text.replace('\t', " ").replace(['\r', '\n'], " ")
}

fn run_protocol(input: &Path, output: &Path) -> Result<(), String> {
    let workloads = parse_protocol(input)?;
    let context = Context::new()?;
    let mut lines = vec![format!("META\tz3_version\t{}", z3_version())];
    lines.push("META\tprotocol\tumlaut-ground-theory-ffi-v1".to_owned());

    for workload in workloads {
        let solver = context.solver()?;
        let declarations = declarations(&workload);
        solver.add_script(&(declarations.clone() + &assertion_script(&workload.base)))?;
        for branch in &workload.branches {
            solver.push();
            solver.add_script(&assertion_script(&branch.assertions))?;
            let started = Instant::now();
            let status_code = solver.check_assumptions(
                workload
                    .base
                    .iter()
                    .chain(branch.assertions.iter())
                    .map(|assertion| assertion.label.as_str()),
            )?;
            let (status, core, model, reason) = match status_code {
                Z3_L_FALSE => (
                    "unsat",
                    solver.core()?.join(","),
                    String::new(),
                    String::new(),
                ),
                Z3_L_TRUE => (
                    "sat",
                    String::new(),
                    solver.model(&workload.variables, &workload.sort)?,
                    String::new(),
                ),
                Z3_L_UNDEF => (
                    "unknown",
                    String::new(),
                    String::new(),
                    solver.reason_unknown(),
                ),
                other => return Err(format!("unrecognized Z3_lbool value: {other}")),
            };
            let elapsed = started.elapsed().as_nanos();
            solver.pop();
            lines.push(format!(
                "RESULT\t{}\t{}\t{status}\t{elapsed}\t{}\t{}\t{}",
                workload.id,
                branch.id,
                sanitize_field(&core),
                sanitize_field(&model),
                sanitize_field(&reason)
            ));
        }
    }
    fs::write(output, lines.join("\n") + "\n").map_err(|error| error.to_string())
}

fn pigeonhole_script(pigeons: usize, holes: usize) -> String {
    let mut script = String::new();
    for pigeon in 0..pigeons {
        for hole in 0..holes {
            script.push_str(&format!("(declare-const p_{pigeon}_{hole} Bool)\n"));
        }
    }
    for pigeon in 0..pigeons {
        script.push_str("(assert (or");
        for hole in 0..holes {
            script.push_str(&format!(" p_{pigeon}_{hole}"));
        }
        script.push_str("))\n");
    }
    for hole in 0..holes {
        for left in 0..pigeons {
            for right in (left + 1)..pigeons {
                script.push_str(&format!(
                    "(assert (or (not p_{left}_{hole}) (not p_{right}_{hole})))\n"
                ));
            }
        }
    }
    script
}

fn run_cancel_probe(output: &Path) -> Result<(), String> {
    let context = Context::new()?;
    let solver = context.solver()?;
    solver.add_script(&pigeonhole_script(70, 69))?;
    let context_address = context.raw as usize;
    let interrupter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(2));
        // SAFETY: Z3_interrupt is the documented cross-thread cancellation
        // operation. The parent joins this thread before dropping the context.
        unsafe {
            Z3_interrupt(context_address as Z3Context);
        }
    });
    let started = Instant::now();
    let status_code = solver.check();
    let elapsed = started.elapsed();
    interrupter
        .join()
        .map_err(|_| "interruption thread panicked".to_owned())?;
    let status = match status_code {
        Z3_L_FALSE => "unsat",
        Z3_L_TRUE => "sat",
        Z3_L_UNDEF => "unknown",
        other => return Err(format!("unrecognized Z3_lbool value: {other}")),
    };
    let reason = if status_code == Z3_L_UNDEF {
        solver.reason_unknown()
    } else {
        "completed-before-interrupt".to_owned()
    };
    fs::write(
        output,
        format!(
            "CANCEL\t{status}\t{}\t{}\n",
            elapsed.as_nanos(),
            sanitize_field(&reason)
        ),
    )
    .map_err(|error| error.to_string())?;
    if status != "unknown" || elapsed > Duration::from_secs(1) {
        return Err(format!(
            "interruption gate failed: status={status}, elapsed={elapsed:?}"
        ));
    }
    Ok(())
}

fn z3_version() -> String {
    // SAFETY: Z3_get_full_version returns a static NUL-terminated string.
    unsafe { c_text(Z3_get_full_version()) }
}

fn usage(program: &str) {
    eprintln!("usage: {program} run INPUT OUTPUT");
    eprintln!("       {program} cancel OUTPUT");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().collect();
    match arguments.as_slice() {
        [_, command, input, output] if command == "run" => {
            run_protocol(Path::new(input), Path::new(output)).map_err(io::Error::other)?;
        }
        [_, command, output] if command == "cancel" => {
            run_cancel_probe(Path::new(output)).map_err(io::Error::other)?;
        }
        _ => {
            usage(&arguments[0]);
            return Err(io::Error::other("invalid arguments").into());
        }
    }
    Ok(())
}
