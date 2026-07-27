#[cfg(feature = "instrument-perf-ctr")]
use crate::basics::defines::DEFAULT_COMCHAR_RAW;
#[cfg(feature = "instrument-perf-ctr")]
use crate::basics::os_wrapper::get_usec_clock;
#[cfg(feature = "instrument-perf-ctr")]
use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerfCounter {
    MguTimer,
    SatTimer,
    ParamodTimer,
    PmIndexTimer,
    IndexUnifTimer,
    BwrwTimer,
    BwrwIndexTimer,
    IndexMatchTimer,
    FreqVecTimer,
    FvIndexTimer,
    SubsumeTimer,
    SetSubsumeTimer,
    ClauseEvalTimer,
}

#[cfg(feature = "instrument-perf-ctr")]
const ALL_COUNTERS: [PerfCounter; 13] = [
    PerfCounter::MguTimer,
    PerfCounter::SatTimer,
    PerfCounter::ParamodTimer,
    PerfCounter::PmIndexTimer,
    PerfCounter::IndexUnifTimer,
    PerfCounter::BwrwTimer,
    PerfCounter::BwrwIndexTimer,
    PerfCounter::IndexMatchTimer,
    PerfCounter::FreqVecTimer,
    PerfCounter::FvIndexTimer,
    PerfCounter::SubsumeTimer,
    PerfCounter::SetSubsumeTimer,
    PerfCounter::ClauseEvalTimer,
];

impl PerfCounter {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MguTimer => "MguTimer",
            Self::SatTimer => "SatTimer",
            Self::ParamodTimer => "ParamodTimer",
            Self::PmIndexTimer => "PMIndexTimer",
            Self::IndexUnifTimer => "IndexUnifTimer",
            Self::BwrwTimer => "BWRWTimer",
            Self::BwrwIndexTimer => "BWRWIndexTimer",
            Self::IndexMatchTimer => "IndexMatchTimer",
            Self::FreqVecTimer => "FreqVecTimer",
            Self::FvIndexTimer => "FVIndexTimer",
            Self::SubsumeTimer => "SubsumeTimer",
            Self::SetSubsumeTimer => "SetSubsumeTimer",
            Self::ClauseEvalTimer => "ClauseEvalTimer",
        }
    }
}

#[must_use]
pub fn start(counter: PerfCounter) -> PerfCounterGuard {
    PerfCounterGuard::new(counter)
}

#[cfg(feature = "instrument-perf-ctr")]
pub struct PerfCounterGuard {
    counter: PerfCounter,
}

#[cfg(not(feature = "instrument-perf-ctr"))]
pub struct PerfCounterGuard;

#[cfg(feature = "instrument-perf-ctr")]
impl PerfCounterGuard {
    fn new(counter: PerfCounter) -> Self {
        counter_start_cell(counter).store(get_usec_clock(), Ordering::Relaxed);
        Self { counter }
    }
}

#[cfg(not(feature = "instrument-perf-ctr"))]
impl PerfCounterGuard {
    const fn new(_counter: PerfCounter) -> Self {
        Self
    }
}

#[cfg(feature = "instrument-perf-ctr")]
impl Drop for PerfCounterGuard {
    fn drop(&mut self) {
        let end = get_usec_clock();
        let start = counter_start_cell(self.counter).swap(0, Ordering::Relaxed);
        add_micros(self.counter, end.saturating_sub(start));
    }
}

#[cfg(feature = "instrument-perf-ctr")]
static MGU_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static SAT_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static PARAMOD_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static PM_INDEX_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static INDEX_UNIF_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static BWRW_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static BWRW_INDEX_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static INDEX_MATCH_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static FREQ_VEC_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static FV_INDEX_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static SUBSUME_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static SET_SUBSUME_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static CLAUSE_EVAL_TIMER: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static MGU_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static SAT_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static PARAMOD_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static PM_INDEX_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static INDEX_UNIF_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static BWRW_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static BWRW_INDEX_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static INDEX_MATCH_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static FREQ_VEC_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static FV_INDEX_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static SUBSUME_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static SET_SUBSUME_TIMER_STORE: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "instrument-perf-ctr")]
static CLAUSE_EVAL_TIMER_STORE: AtomicI64 = AtomicI64::new(0);

#[cfg(feature = "instrument-perf-ctr")]
fn counter_cell(counter: PerfCounter) -> &'static AtomicI64 {
    match counter {
        PerfCounter::MguTimer => &MGU_TIMER,
        PerfCounter::SatTimer => &SAT_TIMER,
        PerfCounter::ParamodTimer => &PARAMOD_TIMER,
        PerfCounter::PmIndexTimer => &PM_INDEX_TIMER,
        PerfCounter::IndexUnifTimer => &INDEX_UNIF_TIMER,
        PerfCounter::BwrwTimer => &BWRW_TIMER,
        PerfCounter::BwrwIndexTimer => &BWRW_INDEX_TIMER,
        PerfCounter::IndexMatchTimer => &INDEX_MATCH_TIMER,
        PerfCounter::FreqVecTimer => &FREQ_VEC_TIMER,
        PerfCounter::FvIndexTimer => &FV_INDEX_TIMER,
        PerfCounter::SubsumeTimer => &SUBSUME_TIMER,
        PerfCounter::SetSubsumeTimer => &SET_SUBSUME_TIMER,
        PerfCounter::ClauseEvalTimer => &CLAUSE_EVAL_TIMER,
    }
}

#[cfg(feature = "instrument-perf-ctr")]
fn counter_start_cell(counter: PerfCounter) -> &'static AtomicI64 {
    match counter {
        PerfCounter::MguTimer => &MGU_TIMER_STORE,
        PerfCounter::SatTimer => &SAT_TIMER_STORE,
        PerfCounter::ParamodTimer => &PARAMOD_TIMER_STORE,
        PerfCounter::PmIndexTimer => &PM_INDEX_TIMER_STORE,
        PerfCounter::IndexUnifTimer => &INDEX_UNIF_TIMER_STORE,
        PerfCounter::BwrwTimer => &BWRW_TIMER_STORE,
        PerfCounter::BwrwIndexTimer => &BWRW_INDEX_TIMER_STORE,
        PerfCounter::IndexMatchTimer => &INDEX_MATCH_TIMER_STORE,
        PerfCounter::FreqVecTimer => &FREQ_VEC_TIMER_STORE,
        PerfCounter::FvIndexTimer => &FV_INDEX_TIMER_STORE,
        PerfCounter::SubsumeTimer => &SUBSUME_TIMER_STORE,
        PerfCounter::SetSubsumeTimer => &SET_SUBSUME_TIMER_STORE,
        PerfCounter::ClauseEvalTimer => &CLAUSE_EVAL_TIMER_STORE,
    }
}

#[cfg(feature = "instrument-perf-ctr")]
pub fn add_micros(counter: PerfCounter, micros: i64) {
    counter_cell(counter).fetch_add(micros, Ordering::Relaxed);
}

#[cfg(feature = "instrument-perf-ctr")]
#[must_use]
pub fn elapsed_micros(counter: PerfCounter) -> i64 {
    counter_cell(counter).load(Ordering::Relaxed)
}

#[cfg(feature = "instrument-perf-ctr")]
pub fn reset(counter: PerfCounter) {
    counter_cell(counter).store(0, Ordering::Relaxed);
}

#[cfg(feature = "instrument-perf-ctr")]
#[must_use]
pub fn statistics_string() -> String {
    let mut output = String::new();
    for counter in ALL_COUNTERS {
        output.push_str(&counter_line(counter));
        output.push('\n');
    }
    output
}

#[cfg(feature = "instrument-perf-ctr")]
#[must_use]
pub fn counter_line(counter: PerfCounter) -> String {
    let label = format!("({})", counter.name());
    #[expect(
        clippy::cast_precision_loss,
        reason = "C PERF_CTR_PRINT casts microsecond counters to float seconds"
    )]
    let rounded_micros = elapsed_micros(counter) as f32;
    let seconds = f64::from(rounded_micros) / 1_000_000.0;
    format!("{DEFAULT_COMCHAR_RAW} PC{label:<34} : {seconds:.6}")
}

#[cfg(all(test, feature = "instrument-perf-ctr"))]
mod tests {
    use super::{add_micros, counter_line, reset, statistics_string, PerfCounter};

    #[test]
    fn counter_line_matches_c_perf_ctr_print_shape() {
        reset(PerfCounter::MguTimer);
        add_micros(PerfCounter::MguTimer, 910);

        assert_eq!(
            counter_line(PerfCounter::MguTimer),
            "% PC(MguTimer)                         : 0.000910"
        );
    }

    #[test]
    fn statistics_match_c_counter_names_and_order() {
        let statistics = statistics_string();
        let labels = statistics
            .lines()
            .map(|line| line.split(':').next().unwrap().trim_end())
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            [
                "% PC(MguTimer)",
                "% PC(SatTimer)",
                "% PC(ParamodTimer)",
                "% PC(PMIndexTimer)",
                "% PC(IndexUnifTimer)",
                "% PC(BWRWTimer)",
                "% PC(BWRWIndexTimer)",
                "% PC(IndexMatchTimer)",
                "% PC(FreqVecTimer)",
                "% PC(FVIndexTimer)",
                "% PC(SubsumeTimer)",
                "% PC(SetSubsumeTimer)",
                "% PC(ClauseEvalTimer)",
            ]
        );
    }
}
