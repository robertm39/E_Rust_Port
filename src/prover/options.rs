use crate::inout::commandline::{OptArgType, OptCell};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EProverOption {
    Help,
    Version,
    Verbose,
    Output,
    Silent,
    OutputLevel,
    PrintStatistics,
    PrintDetailedStatistics,
    PrintSaturated,
    PrintSatInfo,
    FilterSaturated,
    ProofObject,
    ProofGraph,
    ProofStatistics,
    FullDerivation,
    ForceDerivation,
    RecordGivenClauses,
    TrainingExamples,
    PclTermsCompressed,
    PclCompact,
    PclShellLevel,
    CpuLimit,
    SoftCpuLimit,
    MemoryLimit,
    ResourcesInfo,
    SelectStrategy,
    PrintStrategy,
    ParseStrategy,
    ProcessedClausesLimit,
    ProcessedSetLimit,
    UnprocessedLimit,
    TotalClauseSetLimit,
    GeneratedLimit,
    TermBankInsertLimit,
    Answers,
    ConjecturesAreQuestions,
    EqnNoInfix,
    FullEquationalRep,
    PrintOrientedEqLitsAsRules,
    LopIn,
    PclOut,
    TptpIn,
    TptpOut,
    TptpFormat,
    TstpIn,
    TstpOut,
    TstpFormat,
    SyntaxOnly,
    PrintFormulas,
    PruneOnly,
    CnfOnly,
    PrintPid,
    PrintVersion,
    RequireNonempty,
    Auto,
    AutoSchedule,
    SerializeSchedule,
    ForcePreprocessingSchedule,
    SatAutoSchedule,
    NoPreprocessing,
    EqUnfoldLimit,
    EqUnfoldMaxClauses,
    NoEqUnfolding,
    GoalDefs,
    GoalSubtermDefs,
    Sine,
    RelPruningLevel,
    PresatSimplify,
    AcHandling,
    AcNonAggressive,
    LiteralSelectionStrategy,
    NoGeneration,
    SelectOnProcessingOnly,
    InheritParamodLiterals,
    InheritGoalParamodLiterals,
    InheritConjectureParamodLiterals,
    SelectionPosMin,
    SelectionPosMax,
    SelectionNegMin,
    SelectionNegMax,
    SelectionAllMin,
    SelectionAllMax,
    SelectionWeightMin,
    PreferInitialClauses,
    ExpertHeuristic,
    FilterOrphansLimit,
    ForwardContractLimit,
    DeleteBadLimit,
    AssumeCompleteness,
    AssumeIncompleteness,
    DisableEqFactoring,
    DisableParamodIntoNegUnits,
    Condense,
    CondenseAggressive,
    DisableGivenClauseForwardContraction,
    SimulParamod,
    OrientedSimulParamod,
    SupersimulParamod,
    OrientedSupersimulParamod,
    SplitClauses,
    SplitMethod,
    SplitAggressive,
    SplitReuseDefs,
    DisequalityDecomposition,
    DisequalityDecompMaxArity,
    SosUsesInputTypes,
    DestructiveEr,
    StrongDestructiveEr,
    DestructiveErAggressive,
    ForwardContextSr,
    ForwardContextSrAggressive,
    BackwardContextSr,
    PreferGeneralDemodulators,
    ForwardDemodLevel,
    DemodUnderLambda,
    StrongRwInst,
    StrongForwardSubsumption,
    SatCheckProcInterval,
    SatCheckGenInterval,
    SatCheckTTInsertInterval,
    SatCheck,
    SatCheckDecisionLimit,
    SatCheckNormalizeConst,
    SatCheckNormalizeUnproc,
    LiftLambdas,
    Watchlist,
    StaticWatchlist,
    NoWatchlistSimplification,
    ForwardSubsumptionAggressive,
    ConventionalSubsumption,
    SubsumptionIndexing,
    FvIndexFeatureTypes,
    FvIndexMaxFeatures,
    FvIndexSlack,
    RewriteBackwardIndex,
    ParamodFromIndex,
    ParamodIntoIndex,
    FingerprintIndex,
    FingerprintNoSizeConstr,
    PdtNoSizeConstr,
    PdtNoAgeConstr,
    DefineWeightFunction,
    DefineHeuristic,
    FreeNumbers,
    FreeObjects,
    DefinitionalCnf,
    FoolUnroll,
    MiniscopeLimit,
    PrintTypes,
    AppEncode,
    ArgCong,
    NegExt,
    PosExt,
    ExtSupMaxDepth,
    InverseRecognition,
    ReplaceInjDefs,
    Bce,
    BceMaxOccs,
    PredElim,
    PredElimRecognizeGates,
    PredElimForceMuDecrease,
    PredElimIgnoreConjSyms,
    PredElimMaxOccs,
    PredElimTolerance,
    CnfLambdaToForall,
    EtaNormalize,
    HoOrderKind,
    EliminateLeibnizEq,
    UnrollFormulasOnly,
    PrimEnumMode,
    PrimEnumMaxDepth,
    InstChoiceMaxDepth,
    LocalRw,
    PruneArgs,
    FuncProjLimit,
    ImitLimit,
    IdentLimit,
    ElimLimit,
    UnifMode,
    PatternOracle,
    FixpointOracle,
    MaxUnifiers,
    MaxUnifSteps,
    ClassificationTimeoutPortion,
    PreinstantiateInduction,
    TermOrdering,
    OrderWeightGeneration,
    OrderWeights,
    OrderPrecedenceGeneration,
    PrecPureConj,
    PrecConjAxiom,
    PrecPureAxiom,
    PrecSkolem,
    PrecDefPred,
    OrderConstantWeight,
    Precedence,
    LpoRecursionLimit,
    RestrictLiteralComparisons,
    LiteralComparison,
    KboLambdaWeight,
    KboDbWeight,
    DeterministicRewriteSort,
    DeterministicNewSort,
}

pub const EPROVER_OPTIONS: &[OptCell<EProverOption>] = &[
    OptCell::new(
        EProverOption::Help,
        Some('h'),
        Some("help"),
        OptArgType::NoArg,
        None,
        "Print a short description of program usage and options.",
    ),
    OptCell::new(
        EProverOption::Version,
        Some('V'),
        Some("version"),
        OptArgType::NoArg,
        None,
        "Print the version number of the prover. Please include this with all bug reports (if any).",
    ),
    OptCell::new(
        EProverOption::Verbose,
        Some('v'),
        Some("verbose"),
        OptArgType::OptArg,
        Some("1"),
        "Verbose comments on the progress of the program. This differs from the output level (below) in that technical information is printed to stderr, while the output level determines which logical manipulations of the clauses are printed to stdout.",
    ),
    OptCell::new(
        EProverOption::Output,
        Some('o'),
        Some("output-file"),
        OptArgType::ReqArg,
        None,
        "Redirect output into the named file.",
    ),
    OptCell::new(
        EProverOption::Silent,
        Some('s'),
        Some("silent"),
        OptArgType::NoArg,
        None,
        "Equivalent to --output-level=0.",
    ),
    OptCell::new(
        EProverOption::OutputLevel,
        Some('l'),
        Some("output-level"),
        OptArgType::ReqArg,
        None,
        "Select an output level, greater values imply more verbose output. Level 0 produces nearly no output, level 1 will output each clause as it is processed, level 2 will output generating inferences, level 3 will give a full protocol including rewrite steps and level 4 will include some internal clause renamings. Levels >= 2 also imply PCL2 or TSTP formats (which can be post-processed with suitable tools).",
    ),
    OptCell::new(
        EProverOption::ProofObject,
        Some('p'),
        Some("proof-object"),
        OptArgType::OptArg,
        Some("1"),
        "Generate (and print, in case of success) an internal proof object. Level 0 will not print a proof object, level 1 will build asimple, compact proof object that only contains inference rules and dependencies, level 2 will build a proof object where inferences are unambiguously described by giving inference positions, and level 3 will expand this to a proof object where all intermediate results are explicit. This feature is under development, so far only level 0 and 1 are operational. The proof object will be provided in TPTP-3 or PCL syntax, depending on input format and explicit settings. The --proof-graph option will suppress normal output of the proof object in favour of a graphial representation.",
    ),
    OptCell::new(
        EProverOption::ProofGraph,
        None,
        Some("proof-graph"),
        OptArgType::OptArg,
        Some("3"),
        "Generate (and print, in case of success) an internal proof object in the form of a GraphViz dot graph. The optional argument can be 1 (nodes are labelled with just the name of the clause/formula), 2 (nodes are labelled with the TPTP clause/formula) or 3  (nodes also labelled with source/inference record.",
    ),
    OptCell::new(
        EProverOption::ProofStatistics,
        None,
        Some("proof-statistics"),
        OptArgType::NoArg,
        None,
        "Print various statistics of the proof object.",
    ),
    OptCell::new(
        EProverOption::FullDerivation,
        Some('d'),
        Some("full-deriv"),
        OptArgType::NoArg,
        None,
        "Include all derived formuas/clauses in the proof graph/proof object, not just the ones contributing to the actual proof.",
    ),
    OptCell::new(
        EProverOption::ForceDerivation,
        None,
        Some("force-deriv"),
        OptArgType::OptArg,
        Some("1"),
        "Force output of the derivation even in cases where the prover terminates in an indeterminate state. By default, the deriviation of all processed clauses is included in the derivation object. With argument 2, the derivation of all clauses will be printed.",
    ),
    OptCell::new(
        EProverOption::RecordGivenClauses,
        None,
        Some("record-gcs"),
        OptArgType::NoArg,
        None,
        "Record given-clause selection as separate (pseudo-)inferences and preserve the form of given clauses evaluated and selected via archiving for analysis and possibly machine learning.",
    ),
    OptCell::new(
        EProverOption::TrainingExamples,
        None,
        Some("training-examples"),
        OptArgType::OptArg,
        Some("1"),
        "Generate and process training examples from the proof search object. Implies --record-gcs. The argument is a binary or of the desired processing. Bit zero prints positive exampels. Bit 1 prints negative examples. Additional selectors will be added later.",
    ),
    OptCell::new(
        EProverOption::PclTermsCompressed,
        None,
        Some("pcl-terms-compressed"),
        OptArgType::NoArg,
        None,
        "Print terms in the PCL output in shared representation.",
    ),
    OptCell::new(
        EProverOption::PclCompact,
        None,
        Some("pcl-compact"),
        OptArgType::NoArg,
        None,
        "Print PCL steps without additional spaces for formatting (safes disk space for large protocols).",
    ),
    OptCell::new(
        EProverOption::PclShellLevel,
        None,
        Some("pcl-shell-level"),
        OptArgType::OptArg,
        Some("1"),
        "Determines level to which clauses and formulas are suppressed in the output. Level 0 will print all, level 1 will only print initial clauses/formulas, level 2 will print no clauses or axioms. All levels will still print the dependency graph.",
    ),
    OptCell::new(
        EProverOption::PrintStatistics,
        None,
        Some("print-statistics"),
        OptArgType::NoArg,
        None,
        "Print the inference statistics (only relevant for output level <=1, otherwise they are printed automatically.",
    ),
    OptCell::new(
        EProverOption::PrintDetailedStatistics,
        Some('0'),
        Some("print-detailed-statistics"),
        OptArgType::NoArg,
        None,
        "Print data about the proof state that is potentially expensive to collect. Includes number of term cells and number of rewrite steps. This implies the previous option.",
    ),
    OptCell::new(
        EProverOption::PrintSaturated,
        Some('S'),
        Some("print-saturated"),
        OptArgType::OptArg,
        Some("eigEIG"),
        "Print the (semi-) saturated clause sets after terminating the saturation process. The argument given describes which parts should be printed in which order. Legal characters are 'teigEIGaA', standing for type declarations, processed positive units, processed negative units, processed non-units, unprocessed positive units, unprocessed negative units, unprocessed non-units, and two types of additional equality axioms, respectively. Equality axioms will only be printed if the original specification contained real equality. In this case, 'a' requests axioms in which a separate substitutivity axiom is given for each argument position of a function or predicate symbol, while 'A' requests a single substitutivity axiom (covering all positions) for each symbol.",
    ),
    OptCell::new(
        EProverOption::PrintSatInfo,
        None,
        Some("print-sat-info"),
        OptArgType::NoArg,
        None,
        "Print additional information (clause number, weight, etc) as a comment for clauses from the semi-saturated end system.",
    ),
    OptCell::new(
        EProverOption::FilterSaturated,
        None,
        Some("filter-saturated"),
        OptArgType::OptArg,
        Some("Fc"),
        "Filter the  (semi-) saturated clause sets after terminating the saturation process. The argument is a string describing which operations to take (and in which order). Options are 'u' (remove all clauses with more than one literal), 'c' (delete all but one copy of identical clauses, 'n', 'r', 'f' (forward contraction, unit-subsumption only, no rewriting, rewriting with rules only, full rewriting, respectively), and 'N', 'R' and 'F' (as their lower case counterparts, but with non-unit-subsumption enabled as well).",
    ),
    OptCell::new(
        EProverOption::SyntaxOnly,
        None,
        Some("syntax-only"),
        OptArgType::NoArg,
        None,
        "Stop after parsing, i.e. only check if the input can be parsed correcly.",
    ),
    OptCell::new(
        EProverOption::PrintFormulas,
        None,
        Some("print-formulas"),
        OptArgType::NoArg,
        None,
        "If the syntax checks out, print back an include-expanded all formula-version of the peoblem, then terminate.",
    ),
    OptCell::new(
        EProverOption::PruneOnly,
        None,
        Some("prune"),
        OptArgType::NoArg,
        None,
        "Stop after relevancy pruning, SInE pruning, and output of the initial clause- and formula set. This will automatically set output level to 4 so that the pruned problem specification is printed. Note that the desired pruning methods must still be specified (e.g. '--sine=Auto').",
    ),
    OptCell::new(
        EProverOption::CnfOnly,
        None,
        Some("cnf"),
        OptArgType::NoArg,
        None,
        "Convert the input problem into clause normal form and print it. This is (nearly) equivalent to '--print-saturated=eigEIG --processed-clauses-limit=0' and will by default perform some usually useful simplifications. You can additionally specify e.g. '--no-preprocessing' if you want just the result of CNF translation.",
    ),
    OptCell::new(
        EProverOption::PrintPid,
        None,
        Some("print-pid"),
        OptArgType::NoArg,
        None,
        "Print the process id of the prover as a comment after option processing.",
    ),
    OptCell::new(
        EProverOption::PrintVersion,
        None,
        Some("print-version"),
        OptArgType::NoArg,
        None,
        "Print the version number of the prover as a comment after option processing. Note that unlike -version, the prover will not terminate, but proceed normally.",
    ),
    OptCell::new(
        EProverOption::RequireNonempty,
        None,
        Some("error-on-empty"),
        OptArgType::NoArg,
        None,
        "Return with an error code if the input file contains no clauses. Formally, the empty clause set (as an empty conjunction of clauses) is trivially satisfiable, and Umlaut will treat any empty input set as satisfiable. However, in composite systems this is more often a sign that something went wrong. Use this option to catch such bugs.",
    ),
    OptCell::new(
        EProverOption::MemoryLimit,
        Some('m'),
        Some("memory-limit"),
        OptArgType::ReqArg,
        None,
        "Limit the memory the prover may use. The argument is the allowed amount of memory in MB. If you use the argument 'Auto', the system will try to figure out the amount of physical memory of your machine and claim most of it. This option may not work everywhere, due to broken and/or strange behaviour of setrlimit() in some UNIX implementations, and due to the fact that I know of no portable way to figure out the physical memory in a machine. Both the option and the 'Auto' version do work under all tested versions of Solaris and GNU/Linux. Due to problems with limit data types, it is currently impossible to set a limit of more than 2 GB (2048 MB).",
    ),
    OptCell::new(
        EProverOption::CpuLimit,
        None,
        Some("cpu-limit"),
        OptArgType::OptArg,
        Some("300"),
        "Limit the (per core) cpu time the prover should run. The optional argument is the CPU time in seconds. The prover will terminate immediately after reaching the time limit, regardless of internal state. As a side effect, this option will inhibit core file writing. Please note that if you use both --cpu-limit and --soft-cpu-limit, the soft limit has to be smaller than the hard limit to have any effect. ",
    ),
    OptCell::new(
        EProverOption::SoftCpuLimit,
        None,
        Some("soft-cpu-limit"),
        OptArgType::OptArg,
        Some("290"),
        "Limit the cpu time the prover should spend in the main saturation phase. The prover will then terminate gracefully, i.e. it will perform post-processing, filtering and printing of unprocessed clauses, if these options are selected. Note that for some filtering options (in particular those which perform full subsumption), the post-processing time may well be larger than the saturation time. This option is particularly useful if you want to use Umlaut as a preprocessor or lemma generator in a larger system.",
    ),
    OptCell::new(
        EProverOption::ResourcesInfo,
        Some('R'),
        Some("resources-info"),
        OptArgType::NoArg,
        None,
        "Give some information about the resources used by the prover. You will usually get CPU time information. On systems returning more information with the rusage() system call, you will also get information about memory consumption.",
    ),
    OptCell::new(
        EProverOption::SelectStrategy,
        None,
        Some("select-strategy"),
        OptArgType::ReqArg,
        None,
        "Select one of the built-in strategies and set all proof search parameters accordingly.",
    ),
    OptCell::new(
        EProverOption::PrintStrategy,
        None,
        Some("print-strategy"),
        OptArgType::OptArg,
        Some(">current-strategy<"),
        "Print a representation of all search parameters and their setting of a given strategy, then terminate. If no argument is given, the current strategy is printed. Use the reserved name '>all-strats<'to get a description of all built-in strategies,  '>all-names<' to get a list of all names of strategies.",
    ),
    OptCell::new(
        EProverOption::ParseStrategy,
        None,
        Some("parse-strategy"),
        OptArgType::ReqArg,
        None,
        "Parse the previously printed representation of strategy and set all proof search parameters accordingly.",
    ),
    OptCell::new(
        EProverOption::ProcessedClausesLimit,
        Some('C'),
        Some("processed-clauses-limit"),
        OptArgType::ReqArg,
        None,
        "Set the maximal number of clauses to process (i.e. the number of traversals of the main-loop).",
    ),
    OptCell::new(
        EProverOption::ProcessedSetLimit,
        Some('P'),
        Some("processed-set-limit"),
        OptArgType::ReqArg,
        None,
        "Set the maximal size of the set of processed clauses. This differs from the previous option in that redundant and back-simplified processed clauses are not counted.",
    ),
    OptCell::new(
        EProverOption::UnprocessedLimit,
        Some('U'),
        Some("unprocessed-limit"),
        OptArgType::ReqArg,
        None,
        "Set the maximal size of the set of unprocessed clauses. This is a termination condition, not something to use to control the deletion of bad clauses. Compare --delete-bad-limit.",
    ),
    OptCell::new(
        EProverOption::TotalClauseSetLimit,
        Some('T'),
        Some("total-clause-set-limit"),
        OptArgType::ReqArg,
        None,
        "Set the maximal size of the set of all clauses. See previous option.",
    ),
    OptCell::new(
        EProverOption::GeneratedLimit,
        None,
        Some("generated-limit"),
        OptArgType::ReqArg,
        None,
        "Set the maximal number of generated clauses before the proof search stops. This is a reasonable (though not great) estimate of the work done.",
    ),
    OptCell::new(
        EProverOption::TermBankInsertLimit,
        None,
        Some("tb-insert-limit"),
        OptArgType::ReqArg,
        None,
        "Set the maximal number of of term bank term top insertions. This is a reasonable (though not great) estimate of the work done.",
    ),
    OptCell::new(
        EProverOption::Answers,
        None,
        Some("answers"),
        OptArgType::OptArg,
        Some("2147483647"),
        "Set the maximal number of answers to print for existentially quantified questions. Without this option, the prover terminates after the first answer found. If the value is different from 1, the prover is no longer guaranteed to terminate, even if there is a finite number of answers.",
    ),
    OptCell::new(
        EProverOption::ConjecturesAreQuestions,
        None,
        Some("conjectures-are-questions"),
        OptArgType::NoArg,
        None,
        "Treat all conjectures as questions to be answered. This is a wart necessary because CASC-J6 has categories requiring answers, but does not yet support the 'question' type for formulas.",
    ),
    OptCell::new(
        EProverOption::EqnNoInfix,
        Some('n'),
        Some("eqn-no-infix"),
        OptArgType::NoArg,
        None,
        "In LOP, print equations in prefix notation equal(x,y).",
    ),
    OptCell::new(
        EProverOption::FullEquationalRep,
        Some('e'),
        Some("full-equational-rep"),
        OptArgType::NoArg,
        None,
        "In LOP. print all literals as equations, even non-equational ones.",
    ),
    OptCell::new(
        EProverOption::PrintOrientedEqLitsAsRules,
        None,
        Some("print-oriented-eqlits-as-rules"),
        OptArgType::NoArg,
        None,
        "Print oriented equational literals as rules, using -> in place of =.",
    ),
    OptCell::new(
        EProverOption::LopIn,
        None,
        Some("lop-in"),
        OptArgType::NoArg,
        None,
        "Set E-LOP as the input format. If no input format is selected by this or one of the following options, Umlaut will guess the input format based on the first token. It will almost always correctly recognize TPTP-3, but it may misidentify E-LOP files that use TPTP meta-identifiers as logical symbols.",
    ),
    OptCell::new(
        EProverOption::PclOut,
        None,
        Some("pcl-out"),
        OptArgType::NoArg,
        None,
        "Set PCL as the proof object output format.",
    ),
    OptCell::new(
        EProverOption::TptpIn,
        None,
        Some("tptp-in"),
        OptArgType::NoArg,
        None,
        "Set TPTP-2 as the input format (but note that includes are still handled according to TPTP-3 semantics).",
    ),
    OptCell::new(
        EProverOption::TptpOut,
        None,
        Some("tptp-out"),
        OptArgType::NoArg,
        None,
        "Print TPTP format instead of E-LOP. Implies --eqn-no-infix and will ignore --full-equational-rep.",
    ),
    OptCell::new(
        EProverOption::TptpFormat,
        None,
        Some("tptp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tptp-in and --tptp-out.",
    ),
    OptCell::new(
        EProverOption::TptpIn,
        None,
        Some("tptp2-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-in.",
    ),
    OptCell::new(
        EProverOption::TptpOut,
        None,
        Some("tptp2-out"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-out.",
    ),
    OptCell::new(
        EProverOption::TptpFormat,
        None,
        Some("tptp2-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-format.",
    ),
    OptCell::new(
        EProverOption::TstpIn,
        None,
        Some("tstp-in"),
        OptArgType::NoArg,
        None,
        "Set TPTP-3 as the input format. TPTP syntax continues to evolve, and any given Umlaut version may not support every extension. Umlaut supports the TPTP 8.2.0 FOF and CNF files covered by its compatibility suite (including includes).",
    ),
    OptCell::new(
        EProverOption::TstpOut,
        None,
        Some("tstp-out"),
        OptArgType::NoArg,
        None,
        "Print output clauses in TPTP-3 syntax. In particular, for output levels >=2, write derivations as TPTP-3 derivations.",
    ),
    OptCell::new(
        EProverOption::TstpFormat,
        None,
        Some("tstp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-in and --tstp-out.",
    ),
    OptCell::new(
        EProverOption::TstpIn,
        None,
        Some("tptp3-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-in.",
    ),
    OptCell::new(
        EProverOption::TstpOut,
        None,
        Some("tptp3-out"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-out.",
    ),
    OptCell::new(
        EProverOption::TstpFormat,
        None,
        Some("tptp3-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-format.",
    ),
    OptCell::new(
        EProverOption::Auto,
        None,
        Some("auto"),
        OptArgType::NoArg,
        None,
        "Automatically determine settings for proof search.",
    ),
    OptCell::new(
        EProverOption::AutoSchedule,
        None,
        Some("auto-schedule"),
        OptArgType::OptArg,
        Some("1"),
        "Use the (experimental) strategy scheduling. This will try several different fully specified search strategies (aka \"Auto-Modes\"), one after the other, until a proof or saturation is found, or the time limit is exceeded. The optional argument is the number of CPUs on which the schedule is going to be executed on. By default, the schedule is executed on a single core. To execute on all cores of a system, set the argument to 'Auto', but note that this will use all reported cores (even low-performance efficiency cores, if available on the hardware platform and reported by the OS).",
    ),
    OptCell::new(
        EProverOption::ForcePreprocessingSchedule,
        None,
        Some("force-preproc-sched"),
        OptArgType::ReqArg,
        None,
        "When autoscheduling is used, make sure that preprocessing schedule is inserted in the search categories",
    ),
    OptCell::new(
        EProverOption::SerializeSchedule,
        None,
        Some("serialize-schedule"),
        OptArgType::ReqArg,
        None,
        "Convert parallel auto-schedule into serialized one.",
    ),
    OptCell::new(
        EProverOption::SatAutoSchedule,
        None,
        Some("satauto-schedule"),
        OptArgType::OptArg,
        Some("1"),
        "Use strategy scheduling without SInE, thus maintaining completeness.",
    ),
    OptCell::new(
        EProverOption::NoPreprocessing,
        None,
        Some("no-preprocessing"),
        OptArgType::NoArg,
        None,
        "Do not perform preprocessing on the initial clause set. Preprocessing currently removes tautologies and orders terms, literals and clauses in a certain (\"canonical\") way before anything else happens. Unless limited by one of the following options, it will also unfold equational definitions.",
    ),
    OptCell::new(
        EProverOption::EqUnfoldLimit,
        None,
        Some("eq-unfold-limit"),
        OptArgType::ReqArg,
        None,
        "During preprocessing, limit unfolding (and removing) of equational definitions to those where the expanded definition is at most the given limit bigger (in terms of standard weight) than the defined term.",
    ),
    OptCell::new(
        EProverOption::EqUnfoldMaxClauses,
        None,
        Some("eq-unfold-maxclauses"),
        OptArgType::ReqArg,
        None,
        "During preprocessing, don't try unfolding of equational definitions if the problem has more than this limit of clauses.",
    ),
    OptCell::new(
        EProverOption::NoEqUnfolding,
        None,
        Some("no-eq-unfolding"),
        OptArgType::NoArg,
        None,
        "During preprocessing, abstain from unfolding (and removing) equational definitions.",
    ),
    OptCell::new(
        EProverOption::GoalDefs,
        None,
        Some("goal-defs"),
        OptArgType::OptArg,
        Some("All"),
        "Introduce Twee-style equational definitions for ground terms in conjecture clauses. The argument can be None, All or Neg, which will only consider ground terms from negative literals in the CNF (to be implemented).",
    ),
    OptCell::new(
        EProverOption::GoalSubtermDefs,
        None,
        Some("goal-subterm-defs"),
        OptArgType::NoArg,
        None,
        "Introduce goal definitions for all conjecture ground subterms. The default is to only introduce them for the maximal (with respect to the subterm relation) ground terms in conjecture clauses (to be implemented).",
    ),
    OptCell::new(
        EProverOption::Sine,
        None,
        Some("sine"),
        OptArgType::OptArg,
        Some("Auto"),
        "Apply SInE to prune the unprocessed axioms with the specified filter. 'Auto' will automatically pick a filter.",
    ),
    OptCell::new(
        EProverOption::RelPruningLevel,
        None,
        Some("rel-pruning-level"),
        OptArgType::OptArg,
        Some("3"),
        "Perform relevancy pruning up to the given level on the unprocessed axioms.",
    ),
    OptCell::new(
        EProverOption::PresatSimplify,
        None,
        Some("presat-simplify"),
        OptArgType::OptArg,
        Some("true"),
        "Before proper saturation do a complete interreduction of the proof state.",
    ),
    OptCell::new(
        EProverOption::AcHandling,
        None,
        Some("ac-handling"),
        OptArgType::OptArg,
        Some("KeepUnits"),
        "Select AC handling mode, i.e. determine what to do with redundant AC tautologies. The default is equivalent to 'DiscardAll', the other possible values are 'None' (to disable AC handling), 'KeepUnits', and 'KeepOrientable'.",
    ),
    OptCell::new(
        EProverOption::AcNonAggressive,
        None,
        Some("ac-non-aggressive"),
        OptArgType::NoArg,
        None,
        "Do AC resolution on negative literals only on processing (by default, AC resolution is done after clause creation). Only effective if AC handling is not disabled.",
    ),
    OptCell::new(
        EProverOption::LiteralSelectionStrategy,
        Some('W'),
        Some("literal-selection-strategy"),
        OptArgType::ReqArg,
        None,
        "Choose a strategy for selection of negative literals. There are two special values for this option: NoSelection will select no literal (i.e. perform normal superposition) and NoGeneration will inhibit all generating inferences. For a list of the other (hopefully self-documenting) values run 'umlaut -W none'. There are two variants of each strategy. The one prefixed with 'P' will allow paramodulation into maximal positive literals in addition to paramodulation into maximal selected negative literals.",
    ),
    OptCell::new(
        EProverOption::NoGeneration,
        None,
        Some("no-generation"),
        OptArgType::NoArg,
        None,
        "Don't perform any generating inferences (equivalent to --literal-selection-strategy=NoGeneration).",
    ),
    OptCell::new(
        EProverOption::SelectOnProcessingOnly,
        None,
        Some("select-on-processing-only"),
        OptArgType::NoArg,
        None,
        "Perform literal selection at processing time only (i.e. select only in the _given clause_), not before clause evaluation. This is relevant because many clause selection heuristics give special consideration to maximal or selected literals.",
    ),
    OptCell::new(
        EProverOption::InheritParamodLiterals,
        Some('i'),
        Some("inherit-paramod-literals"),
        OptArgType::NoArg,
        None,
        "Always select the negative literals a previous inference paramodulated into (if possible). If no such literal exists, select as dictated by the selection strategy.",
    ),
    OptCell::new(
        EProverOption::InheritGoalParamodLiterals,
        Some('j'),
        Some("inherit-goal-pm-literals"),
        OptArgType::NoArg,
        None,
        "In a goal (all negative clause), always select the negative literals a previous inference paramodulated into (if possible). If no such literal exists, select as dictated by the selection strategy.",
    ),
    OptCell::new(
        EProverOption::InheritConjectureParamodLiterals,
        None,
        Some("inherit-conjecture-pm-literals"),
        OptArgType::NoArg,
        None,
        "In a conjecture-derived clause, always select the negative literals a previous inference paramodulated into (if possible). If no such literal exists, select as dictated by the selection strategy.",
    ),
    OptCell::new(
        EProverOption::SelectionPosMin,
        None,
        Some("selection-pos-min"),
        OptArgType::ReqArg,
        None,
        "Set a lower limit for the number of positive literals a clause must have to be eligible for literal selection.",
    ),
    OptCell::new(
        EProverOption::SelectionPosMax,
        None,
        Some("selection-pos-max"),
        OptArgType::ReqArg,
        None,
        "Set a upper limit for the number of positive literals a clause can have to be eligible for literal selection.",
    ),
    OptCell::new(
        EProverOption::SelectionNegMin,
        None,
        Some("selection-neg-min"),
        OptArgType::ReqArg,
        None,
        "Set a lower limit for the number of negative literals a clause must have to be eligible for literal selection.",
    ),
    OptCell::new(
        EProverOption::SelectionNegMax,
        None,
        Some("selection-neg-max"),
        OptArgType::ReqArg,
        None,
        "Set a upper limit for the number of negative literals a clause can have to be eligible for literal selection.",
    ),
    OptCell::new(
        EProverOption::SelectionAllMin,
        None,
        Some("selection-all-min"),
        OptArgType::ReqArg,
        None,
        "Set a lower limit for the number of literals a clause must have to be eligible for literal selection.",
    ),
    OptCell::new(
        EProverOption::SelectionAllMax,
        None,
        Some("selection-all-max"),
        OptArgType::ReqArg,
        None,
        "Set an upper limit for the number of literals a clause must have to be eligible for literal selection.",
    ),
    OptCell::new(
        EProverOption::SelectionWeightMin,
        None,
        Some("selection-weight-min"),
        OptArgType::ReqArg,
        None,
        "Set the minimum weight a clause must have to be eligible for literal selection.",
    ),
    OptCell::new(
        EProverOption::PreferInitialClauses,
        None,
        Some("prefer-initial-clauses"),
        OptArgType::NoArg,
        None,
        "Always process all initial clauses first.",
    ),
    OptCell::new(
        EProverOption::ExpertHeuristic,
        Some('x'),
        Some("expert-heuristic"),
        OptArgType::ReqArg,
        None,
        "Select one of the clause selection heuristics. Currently at least available: Auto, Weight, StandardWeight, RWeight, FIFO, LIFO, Uniq, UseWatchlist. For a full list check HEURISTICS/che_proofcontrol.c. Auto is recommended if you only want to find a proof. It is special in that it will also set some additional options. To have optimal performance, you also should specify -tAuto to select a good term ordering. LIFO is unfair and will make the prover incomplete. Uniq is used internally and is not very useful in most cases. You can define more heuristics using the option -H (see below).",
    ),
    OptCell::new(
        EProverOption::FilterOrphansLimit,
        None,
        Some("filter-orphans-limit"),
        OptArgType::OptArg,
        Some("100"),
        "Orphans are unprocessed clauses where one of the parents has been removed by back-simolification. They are redundant and usually removed lazily (i.e. only when they are selected for processing). With this option you can select a limit on back-simplified clauses  after which orphans will be eagerly deleted.",
    ),
    OptCell::new(
        EProverOption::ForwardContractLimit,
        None,
        Some("forward-contract-limit"),
        OptArgType::OptArg,
        Some("80000"),
        "Set a limit on the number of processed clauses after which the unprocessed clause set will be re-simplified and reweighted. ",
    ),
    OptCell::new(
        EProverOption::DeleteBadLimit,
        None,
        Some("delete-bad-limit"),
        OptArgType::OptArg,
        Some("1500000"),
        "Set the number of storage units after which bad clauses are deleted without further consideration. This causes the prover to be potentially incomplete, but will allow you to limit the maximum amount of memory used fairly well. The prover will tell you if a proof attempt failed due to the incompleteness introduced by this option. It is recommended to set this limit significantly higher than --filter-limit or --filter-copies-limit. If you select -xAuto and set a memory limit, the prover will determine a good value automatically.",
    ),
    OptCell::new(
        EProverOption::AssumeCompleteness,
        None,
        Some("assume-completeness"),
        OptArgType::NoArg,
        None,
        "There are various way (e.g. the next few options) to configure the prover to be strongly incomplete in the general case. Umlaut will detect when such an option is selected and return corresponding exit states (i.e. it will not claim satisfiability just because it ran out of unprocessed clauses). If you _know_ that for your class of problems the selected strategy is still complete, use this option to tell the system that this is the case.",
    ),
    OptCell::new(
        EProverOption::AssumeIncompleteness,
        None,
        Some("assume-incompleteness"),
        OptArgType::NoArg,
        None,
        "This option instructs the prover to assume incompleteness (typically because the axiomatization already is incomplete because axioms have been filtered before they are handed to the system.",
    ),
    OptCell::new(
        EProverOption::DisableEqFactoring,
        None,
        Some("disable-eq-factoring"),
        OptArgType::NoArg,
        None,
        "Disable equality factoring. This makes the prover incomplete for general non-Horn problems, but helps for some specialized classes. It is not necessary to disable equality factoring for Horn problems, as Horn clauses are not factored anyways.",
    ),
    OptCell::new(
        EProverOption::DisableParamodIntoNegUnits,
        None,
        Some("disable-paramod-into-neg-units"),
        OptArgType::NoArg,
        None,
        "Disable paramodulation into negative unit clause. This makes the prover incomplete in the general case, but helps for some specialized classes.",
    ),
    OptCell::new(
        EProverOption::Condense,
        None,
        Some("condense"),
        OptArgType::NoArg,
        None,
        "Enable condensing for the given clause. Condensing replaces a clause by a more general factor (if such a factor exists).",
    ),
    OptCell::new(
        EProverOption::CondenseAggressive,
        None,
        Some("condense-aggressive"),
        OptArgType::NoArg,
        None,
        "Enable condensing for the given and newly generated clauses.",
    ),
    OptCell::new(
        EProverOption::DisableGivenClauseForwardContraction,
        None,
        Some("disable-given-clause-fw-contraction"),
        OptArgType::NoArg,
        None,
        "Disable simplification and subsumption of the newly selected given clause (clauses are still simplified when they are generated). In general, this breaks some basic assumptions of the DISCOUNT loop proof search procedure. However, there are some problem classes in which  this simplifications empirically never occurs. In such cases, we can save significant overhead. The option _should_ work in all cases, but is not expected to improve things in most cases.",
    ),
    OptCell::new(
        EProverOption::SimulParamod,
        None,
        Some("simul-paramod"),
        OptArgType::NoArg,
        None,
        "Use simultaneous paramodulation to implement superposition. Default is to use plain paramodulation.",
    ),
    OptCell::new(
        EProverOption::OrientedSimulParamod,
        None,
        Some("oriented-simul-paramod"),
        OptArgType::NoArg,
        None,
        "Use simultaneous paramodulation for oriented from-literals. This is an experimental feature.",
    ),
    OptCell::new(
        EProverOption::SupersimulParamod,
        None,
        Some("supersimul-paramod"),
        OptArgType::NoArg,
        None,
        "Use supersimultaneous paramodulation to implement superposition. Default is to use plain paramodulation.",
    ),
    OptCell::new(
        EProverOption::OrientedSupersimulParamod,
        None,
        Some("oriented-supersimul-paramod"),
        OptArgType::NoArg,
        None,
        "Use supersimultaneous paramodulation for oriented from-literals. This is an experimental feature.",
    ),
    OptCell::new(
        EProverOption::SplitClauses,
        None,
        Some("split-clauses"),
        OptArgType::OptArg,
        Some("7"),
        "Determine which clauses should be subject to splitting. The argument is the binary 'OR' of values for the desired classes:\n     1:  Horn clauses\n     2:  Non-Horn clauses\n     4:  Negative clauses\n     8:  Positive clauses\n    16:  Clauses with both positive and negative literals\nEach set bit adds that class to the set of clauses which will be split.",
    ),
    OptCell::new(
        EProverOption::SplitMethod,
        None,
        Some("split-method"),
        OptArgType::ReqArg,
        None,
        "Determine how to treat ground literals in splitting. The argument is either '0' to denote no splitting of ground literals (they are all assigned to the first split clause produced), '1' to denote that all ground literals should form a single new clause, or '2', in which case ground literals are treated as usual and are all split off into individual clauses.",
    ),
    OptCell::new(
        EProverOption::SplitAggressive,
        None,
        Some("split-aggressive"),
        OptArgType::NoArg,
        None,
        "Apply splitting to new clauses (after simplification) and before evaluation. By default, splitting (if activated) is only performed on selected clauses. ",
    ),
    OptCell::new(
        EProverOption::SplitReuseDefs,
        None,
        Some("split-reuse-defs"),
        OptArgType::NoArg,
        None,
        "If possible, reuse previous definitions for splitting.",
    ),
    OptCell::new(
        EProverOption::DisequalityDecomposition,
        None,
        Some("disequality-decomposition"),
        OptArgType::OptArg,
        Some("1024"),
        "Enable the disequality decomposition inference. The optional argument is the maximal literal number of clauses considered for the inference.",
    ),
    OptCell::new(
        EProverOption::DisequalityDecompMaxArity,
        None,
        Some("disequality-decomp-maxarity"),
        OptArgType::OptArg,
        Some("1"),
        "Limit disequality decomposition to function symbols of at most the given arity.",
    ),
    OptCell::new(
        EProverOption::TermOrdering,
        Some('t'),
        Some("term-ordering"),
        OptArgType::ReqArg,
        None,
        "Select an ordering type (currently Auto, LPO, LPO4, KBO or KBO6). -tAuto is suggested, in particular with -xAuto. KBO and KBO6 are different implementations of the same ordering, KBO6 is usually faster and has had more testing. Similarly, LPO4 is a new, equivalent but superior implementation of LPO.",
    ),
    OptCell::new(
        EProverOption::OrderWeightGeneration,
        Some('w'),
        Some("order-weight-generation"),
        OptArgType::ReqArg,
        None,
        "Select a method for the generation of weights for use with the term ordering. Run 'umlaut -w none' for a list of options.",
    ),
    OptCell::new(
        EProverOption::OrderWeights,
        None,
        Some("order-weights"),
        OptArgType::ReqArg,
        None,
        "Describe a (partial) assignments of weights to function symbols for term orderings (in particular, KBO). You can specify a list of weights of the form 'f1:w1,f2:w2, ...'. Since a total weight assignment is needed, Umlaut will _first_ apply any weight generation scheme specified (or the default one), and then modify the weights as specified. Note that Umlaut performs only very basic sanity checks, so you probably can specify weights that break KBO constraints.",
    ),
    OptCell::new(
        EProverOption::OrderPrecedenceGeneration,
        Some('G'),
        Some("order-precedence-generation"),
        OptArgType::ReqArg,
        None,
        "Select a method for the generation of a precedence for use with the term ordering. Run 'umlaut -G none' for a list of options.",
    ),
    OptCell::new(
        EProverOption::PrecPureConj,
        None,
        Some("prec-pure-conj"),
        OptArgType::OptArg,
        Some("10"),
        "Set a weight for symbols that occur in conjectures only to determinewhere to place it in the precedence. This value is used for a roughpre-order, the normal schemes only sort within symbols with the sameoccurrence modifier.",
    ),
    OptCell::new(
        EProverOption::PrecConjAxiom,
        None,
        Some("prec-conj-axiom"),
        OptArgType::OptArg,
        Some("5"),
        "Set a weight for symbols that occur in both conjectures and axiomsto determine where to place it in the precedence. This value is used for a rough pre-order, the normal schemes only sort within symbols with the same occurrence modifier.",
    ),
    OptCell::new(
        EProverOption::PrecPureAxiom,
        None,
        Some("prec-pure-axiom"),
        OptArgType::OptArg,
        Some("2"),
        "Set a weight for symbols that occur in axioms only to determine where to place it in the precedence. This value is used for a rough pre-order, the normal schemes only sort within symbols with the same occurrence modifier.",
    ),
    OptCell::new(
        EProverOption::PrecSkolem,
        None,
        Some("prec-skolem"),
        OptArgType::OptArg,
        Some("2"),
        "Set a weight for Skolem symbols to determine where to place it in the precedence. This value is used for a rough pre-order, the normal schemes only sort within symbols with the same occurrence modifier.",
    ),
    OptCell::new(
        EProverOption::PrecDefPred,
        None,
        Some("prec-defpred"),
        OptArgType::OptArg,
        Some("2"),
        "Set a weight for introduced predicate symbols (usually via definitional CNF or clause splitting) to determine where to place it in the precedence. This value is used for a rough pre-order, the normal schemes only sort within symbols with the same occurrence modifier.",
    ),
    OptCell::new(
        EProverOption::OrderConstantWeight,
        Some('c'),
        Some("order-constant-weight"),
        OptArgType::ReqArg,
        None,
        "Set a special weight > 0 for constants in the term ordering. By default, constants are treated like other function symbols.",
    ),
    OptCell::new(
        EProverOption::Precedence,
        None,
        Some("precedence"),
        OptArgType::OptArg,
        Some(""),
        "Describe a (partial) precedence for the term ordering used for the proof attempt. You can specify a comma-separated list of precedence chains, where a precedence chain is a list of function symbols (which all have to appear in the proof problem), connected by >, <, or =. If this option is used in connection with --order-precedence-generation, the partial ordering will be completed using the selected method, otherwise the prover runs with a non-ground-total ordering.",
    ),
    OptCell::new(
        EProverOption::LpoRecursionLimit,
        None,
        Some("lpo-recursion-limit"),
        OptArgType::OptArg,
        Some("100"),
        "Set a depth limit for LPO comparisons. Most comparisons do not need more than 10 or 20 levels of recursion. By default, recursion depth is limited to 1000 to avoid stack overflow problems. If the limit is reached, the prover assumes that the terms are uncomparable. Smaller values make the comparison attempts faster, but less exact. Larger values have the opposite effect. Values up to 20000 should be save on most operating systems. If you run into segmentation faults while using LPO or LPO4, first try to set this limit to a reasonable value. If the problem persists, send a bug report ;-)",
    ),
    OptCell::new(
        EProverOption::RestrictLiteralComparisons,
        None,
        Some("restrict-literal-comparisons"),
        OptArgType::NoArg,
        None,
        "Make all literals uncomparable in the term ordering (i.e. do not use the term ordering to restrict paramodulation, equality resolution and factoring to certain literals. This is necessary to make Set-of-Support-strategies complete for the non-equational case (It still is incomplete for the equational case, but pretty useless anyways).",
    ),
    OptCell::new(
        EProverOption::LiteralComparison,
        None,
        Some("literal-comparison"),
        OptArgType::ReqArg,
        None,
        "Modify how literal comparisons are done. 'None' is equivalent to the previous option, 'Normal' uses the normal lifting of the term ordering, 'TFOEqMax' uses the equivalent of a transfinite ordering deciding on the predicate symbol and making equational literals maximal (note that this setting makes the prover incomplere), and 'TFOEqMin' modifies this by making equational symbols minimal.",
    ),
    OptCell::new(
        EProverOption::SosUsesInputTypes,
        None,
        Some("sos-uses-input-types"),
        OptArgType::NoArg,
        None,
        "If input is TPTP format, use TPTP conjectures for initializing the Set of Support. If not in TPTP format, use E-LOP queries (clauses of the form ?-l(X),...,m(Y)). Normally, all negative clauses are used. Please note that most E heuristics do not use this information at all, it is currently only useful for certain parameter settings (including the SimulateSOS priority function).",
    ),
    OptCell::new(
        EProverOption::DestructiveEr,
        None,
        Some("destructive-er"),
        OptArgType::NoArg,
        None,
        "Allow destructive equality resolution inferences on pure-variable literals of the form X!=Y, i.e. replace the original clause with the result of an equality resolution inference on this literal.",
    ),
    OptCell::new(
        EProverOption::StrongDestructiveEr,
        None,
        Some("strong-destructive-er"),
        OptArgType::NoArg,
        None,
        "Allow destructive equality resolution inferences on literals of the form X!=t (where X does not occur in t), i.e. replace the original clause with the result of an equality resolution inference on this literal. Unless I am brain-dead, this maintains completeness, although the proof is rather tricky.",
    ),
    OptCell::new(
        EProverOption::DestructiveErAggressive,
        None,
        Some("destructive-er-aggressive"),
        OptArgType::NoArg,
        None,
        "Apply destructive equality resolution to all newly generated clauses, not just to selected clauses. Implies --destructive-er.",
    ),
    OptCell::new(
        EProverOption::ForwardContextSr,
        None,
        Some("forward-context-sr"),
        OptArgType::NoArg,
        None,
        "Apply contextual simplify-reflect with processed clauses to the given clause.",
    ),
    OptCell::new(
        EProverOption::ForwardContextSrAggressive,
        None,
        Some("forward-context-sr-aggressive"),
        OptArgType::NoArg,
        None,
        "Apply contextual simplify-reflect with processed clauses to new clauses. Implies --forward-context-sr.",
    ),
    OptCell::new(
        EProverOption::BackwardContextSr,
        None,
        Some("backward-context-sr"),
        OptArgType::NoArg,
        None,
        "Apply contextual simplify-reflect with the given clause to processed clauses.",
    ),
    OptCell::new(
        EProverOption::PreferGeneralDemodulators,
        Some('g'),
        Some("prefer-general-demodulators"),
        OptArgType::NoArg,
        None,
        "Prefer general demodulators. By default, Umlaut prefers specialized demodulators. This affects in which order the rewrite  index is traversed.",
    ),
    OptCell::new(
        EProverOption::ForwardDemodLevel,
        Some('F'),
        Some("forward-demod-level"),
        OptArgType::ReqArg,
        None,
        "Set the desired level for rewriting of unprocessed clauses. A value of 0 means no rewriting, 1 indicates to use rules (orientable equations) only, 2 indicates full rewriting with rules and instances of unorientable equations. Default behavior is 2.",
    ),
    OptCell::new(
        EProverOption::DemodUnderLambda,
        None,
        Some("demod-under-lambda"),
        OptArgType::ReqArg,
        None,
        "Demodulate *closed* subterms under lambdas.",
    ),
    OptCell::new(
        EProverOption::StrongRwInst,
        None,
        Some("strong-rw-inst"),
        OptArgType::NoArg,
        None,
        "Instantiate unbound variables in matching potential demodulators with a small constant terms.",
    ),
    OptCell::new(
        EProverOption::StrongForwardSubsumption,
        Some('u'),
        Some("strong-forward-subsumption"),
        OptArgType::NoArg,
        None,
        "Try multiple positions and unit-equations to try to equationally subsume a single new clause. Default is to search for a single position.",
    ),
    OptCell::new(
        EProverOption::SatCheckProcInterval,
        None,
        Some("satcheck-proc-interval"),
        OptArgType::OptArg,
        Some("5000"),
        "Enable periodic SAT checking at the given interval of main loop non-trivial processed clauses.",
    ),
    OptCell::new(
        EProverOption::SatCheckGenInterval,
        None,
        Some("satcheck-gen-interval"),
        OptArgType::OptArg,
        Some("10000"),
        "Enable periodic SAT checking whenever the total proof state size increases by the given limit.",
    ),
    OptCell::new(
        EProverOption::SatCheckTTInsertInterval,
        None,
        Some("satcheck-ttinsert-interval"),
        OptArgType::OptArg,
        Some("5000000"),
        "Enable periodic SAT checking whenever the number of term tops insertions matches the given limit (which grows exponentially).",
    ),
    OptCell::new(
        EProverOption::SatCheck,
        None,
        Some("satcheck"),
        OptArgType::OptArg,
        Some("FirstConst"),
        "Set the grounding strategy for periodic SAT checking. Note that to enable SAT checking, it is also necessary to set the interval with one of the previous two options.",
    ),
    OptCell::new(
        EProverOption::SatCheckDecisionLimit,
        None,
        Some("satcheck-decision-limit"),
        OptArgType::OptArg,
        Some("100"),
        "Set the number of decisions allowed for each run of the SAT solver. If the option is not given, the built-in value is 10000. Use -1 to allow unlimited decision.",
    ),
    OptCell::new(
        EProverOption::SatCheckNormalizeConst,
        None,
        Some("satcheck-normalize-const"),
        OptArgType::NoArg,
        None,
        "Use the current normal form (as recorded in the termbank rewrite cache) of the selected constant as the term for the grounding substitution.",
    ),
    OptCell::new(
        EProverOption::SatCheckNormalizeUnproc,
        None,
        Some("satcheck-normalize-unproc"),
        OptArgType::NoArg,
        None,
        "Enable re-simplification (heuristic re-revaluation) of unprocessed clauses before grounding for SAT checking.",
    ),
    OptCell::new(
        EProverOption::Watchlist,
        None,
        Some("watchlist"),
        OptArgType::OptArg,
        Some("'Use inline watchlist type'"),
        "Give the name for a file containing clauses to be watched for during the saturation process. If a clause is generated that subsumes a watchlist clause, the subsumed clause is removed from the watchlist. The prover will terminate when the watchlist is empty. If you want to use the watchlist for guiding the proof, put the empty clause onto the list and use the built-in clause selection heuristic 'UseWatchlist' (or build a heuristic yourself using the priority functions 'PreferWatchlist' and 'DeferWatchlist'). Use the argument 'Use inline watchlist type' (or no argument) and the special clause type 'watchlist' if you want to put watchlist clauses into the normal input stream. This is only supported for TPTP input formats.",
    ),
    OptCell::new(
        EProverOption::StaticWatchlist,
        None,
        Some("static-watchlist"),
        OptArgType::OptArg,
        Some("'Use inline watchlist type'"),
        "This is identical to the previous option, but subsumed clauses willnot be removed from the watchlist (and hence the prover will not terminate if all watchlist clauses have been subsumed. This may be more useful for heuristic guidance.",
    ),
    OptCell::new(
        EProverOption::NoWatchlistSimplification,
        None,
        Some("no-watchlist-simplification"),
        OptArgType::NoArg,
        None,
        "By default, the watchlist is brought into normal form with respect to the current processed clause set and certain simplifications. This option disables simplification for the watchlist.",
    ),
    OptCell::new(
        EProverOption::ForwardSubsumptionAggressive,
        None,
        Some("fw-subsumption-aggressive"),
        OptArgType::NoArg,
        None,
        "Perform forward subsumption on newly generated clauses before they are evaluated. This is particularly useful if heuristic evaluation is very expensive, e.g. via externally connected neural networks.",
    ),
    OptCell::new(
        EProverOption::ConventionalSubsumption,
        None,
        Some("conventional-subsumption"),
        OptArgType::NoArg,
        None,
        "Equivalent to --subsumption-indexing=None.",
    ),
    OptCell::new(
        EProverOption::SubsumptionIndexing,
        None,
        Some("subsumption-indexing"),
        OptArgType::ReqArg,
        None,
        "Determine choice of indexing for (most) subsumption operations. Choices are 'None' for naive subsumption, 'Direct' for direct mapped FV-Indexing, 'Perm' for permuted FV-Indexing and 'PermOpt' for permuted FV-Indexing with deletion of (suspected) non-informative features. Default behaviour is 'Perm'.",
    ),
    OptCell::new(
        EProverOption::FvIndexFeatureTypes,
        None,
        Some("fvindex-featuretypes"),
        OptArgType::ReqArg,
        None,
        "Select the feature types used for indexing. Choices are \"None\" to disable FV-indexing, \"AC\" for AC compatible features (the default) (literal number and symbol counts), \"SS\" for set subsumption compatible features (symbol depth), and \"All\" for all features.Unless you want to measure the effects of the different features, I suggest you stick with the default.",
    ),
    OptCell::new(
        EProverOption::FvIndexMaxFeatures,
        None,
        Some("fvindex-maxfeatures"),
        OptArgType::OptArg,
        Some("200"),
        "Set the maximum initial number of symbols for feature computation. Depending on the feature selection, a value of X here will convert into 2X+2 features (for set subsumption features), 2X+4 features (for AC-compatible features) or 4X+6 features (if all features are used, the default). Note that the actually used set of features may be smaller than this if the signature does not contain enough symbols.For the Perm and PermOpt version, this is _also_ used to set the maximum depth of the feature vector index. Yes, I should probably make this into two separate options. If you select a small value here, you should probably not use \"Direct\" for the --subsumption-indexing option.",
    ),
    OptCell::new(
        EProverOption::FvIndexSlack,
        None,
        Some("fvindex-slack"),
        OptArgType::OptArg,
        Some("0"),
        "Set the number of slots reserved in the index for function symbols that may be introduced into the signature later, e.g. by splitting. If no new symbols are introduced, this just wastes time and memory. If PermOpt is chosen, the slackness slots will be deleted from the index anyways, but will still waste (a little) time in computing feature vectors.",
    ),
    OptCell::new(
        EProverOption::RewriteBackwardIndex,
        None,
        Some("rw-bw-index"),
        OptArgType::OptArg,
        Some("FP7"),
        "Select fingerprint function for backwards rewrite index. \"NoIndex\" will disable paramodulation indexing. For a list of the other values run 'umlaut --pm-index=none'. FPX functions will use a fingerprint of X positions, the letters disambiguate between different fingerprints with the same sample size.",
    ),
    OptCell::new(
        EProverOption::ParamodFromIndex,
        None,
        Some("pm-from-index"),
        OptArgType::OptArg,
        Some("FP7"),
        "Select fingerprint function for the index for paramodulation from indexed clauses. \"NoIndex\" will disable paramodulation indexing. For a list of the other values run 'umlaut --pm-index=none'. FPX functionswill use a fingerprint of X positions, the letters disambiguate between different fingerprints with the same sample size.",
    ),
    OptCell::new(
        EProverOption::ParamodIntoIndex,
        None,
        Some("pm-into-index"),
        OptArgType::OptArg,
        Some("FP7"),
        "Select fingerprint function for the index for paramodulation into the indexed clauses. \"NoIndex\" will disable paramodulation indexing. For a list of the other values run 'umlaut --pm-index=none'. FPX functionswill use a fingerprint of X positions, the letters disambiguate between different fingerprints with the same sample size.",
    ),
    OptCell::new(
        EProverOption::FingerprintIndex,
        None,
        Some("fp-index"),
        OptArgType::OptArg,
        Some("FP7"),
        "Select fingerprint function for all fingerprint indices. See above.",
    ),
    OptCell::new(
        EProverOption::FingerprintNoSizeConstr,
        None,
        Some("fp-no-size-constr"),
        OptArgType::NoArg,
        None,
        "Disable usage of size constraints for matching with fingerprint indexing.",
    ),
    OptCell::new(
        EProverOption::PdtNoSizeConstr,
        None,
        Some("pdt-no-size-constr"),
        OptArgType::NoArg,
        None,
        "Disable usage of size constraints for matching with perfect discrimination trees indexing.",
    ),
    OptCell::new(
        EProverOption::PdtNoAgeConstr,
        None,
        Some("pdt-no-age-constr"),
        OptArgType::NoArg,
        None,
        "Disable usage of age constraints for matching with perfect discrimination trees indexing.",
    ),
    OptCell::new(
        EProverOption::DeterministicRewriteSort,
        None,
        Some("detsort-rw"),
        OptArgType::NoArg,
        None,
        "Sort set of clauses eliminated by backward rewriting using a total syntactic ordering.",
    ),
    OptCell::new(
        EProverOption::DeterministicNewSort,
        None,
        Some("detsort-new"),
        OptArgType::NoArg,
        None,
        "Sort set of newly generated and backward simplified clauses using a total syntactic ordering.",
    ),
    OptCell::new(
        EProverOption::DefineWeightFunction,
        Some('D'),
        Some("define-weight-function"),
        OptArgType::ReqArg,
        None,
        "Define  a weight function (see manual for details). Later definitions override previous definitions.",
    ),
    OptCell::new(
        EProverOption::DefineHeuristic,
        Some('H'),
        Some("define-heuristic"),
        OptArgType::ReqArg,
        None,
        "Define a clause selection heuristic (see manual for details). Later definitions override previous definitions.",
    ),
    OptCell::new(
        EProverOption::FreeNumbers,
        None,
        Some("free-numbers"),
        OptArgType::NoArg,
        None,
        "Treat numbers (strings of decimal digits) as normal free function symbols in the input. By default, number now are supposed to denote domain constants and to be implicitly different from each other.",
    ),
    OptCell::new(
        EProverOption::FreeObjects,
        None,
        Some("free-objects"),
        OptArgType::NoArg,
        None,
        "Treat object identifiers (strings in double quotes) as normal free function symbols in the input. By default, object identifiers now represent domain objects and are implicitly different from each other (and from numbers, unless those are declared to be free).",
    ),
    OptCell::new(
        EProverOption::DefinitionalCnf,
        None,
        Some("definitional-cnf"),
        OptArgType::OptArg,
        Some("24"),
        "Tune the clausification algorithm to introduces definitions for subformulae to avoid exponential blow-up. The optional argument is a fudge factor that determines when definitions are introduced. 0 disables definitions completely. The default works well.",
    ),
    OptCell::new(
        EProverOption::FoolUnroll,
        None,
        Some("fool-unroll"),
        OptArgType::ReqArg,
        None,
        "Enable or disable FOOL unrolling. Useful for some SH problems.",
    ),
    OptCell::new(
        EProverOption::MiniscopeLimit,
        None,
        Some("miniscope-limit"),
        OptArgType::OptArg,
        Some("2147483648"),
        "Set the limit of sub-formula-size to miniscope. The build-indefault is 256. Only applies to the new (default) clausification algorithm",
    ),
    OptCell::new(
        EProverOption::PrintTypes,
        None,
        Some("print-types"),
        OptArgType::NoArg,
        None,
        "Print the type of every term. Useful for debugging purposes.",
    ),
    OptCell::new(
        EProverOption::AppEncode,
        None,
        Some("app-encode"),
        OptArgType::NoArg,
        None,
        "Encodes terms in the proof state using applicative encoding, prints encoded input problem and exits.",
    ),
    OptCell::new(
        EProverOption::ArgCong,
        None,
        Some("arg-cong"),
        OptArgType::ReqArg,
        None,
        "Turns on ArgCong inference rule. Excepts an argument \"all\" or \"max\" that applies the rule to all or only literals that are eligible for resolution.",
    ),
    OptCell::new(
        EProverOption::NegExt,
        None,
        Some("neg-ext"),
        OptArgType::ReqArg,
        None,
        "Turns on NegExt inference rule. Excepts an argument \"all\" or \"max\" that applies the rule to all or only literals that are eligible for resolution.",
    ),
    OptCell::new(
        EProverOption::PosExt,
        None,
        Some("pos-ext"),
        OptArgType::ReqArg,
        None,
        "Turns on PosExt inference rule. Excepts an argument \"all\" or \"max\" that applies the rule to all or only literals that are eligible for resolution.",
    ),
    OptCell::new(
        EProverOption::ExtSupMaxDepth,
        None,
        Some("ext-sup-max-depth"),
        OptArgType::ReqArg,
        None,
        "Sets the maximal proof depth of the clause which will be considered for  Ext-family of inferences. Negative value disables the rule.",
    ),
    OptCell::new(
        EProverOption::InverseRecognition,
        None,
        Some("inverse-recognition"),
        OptArgType::NoArg,
        None,
        "Enables the recognition of injective function symbols. If such a symbol is recognized, existence of the inverse function is asserted by adding a corresponding axiom.",
    ),
    OptCell::new(
        EProverOption::ReplaceInjDefs,
        None,
        Some("replace-inj-defs"),
        OptArgType::NoArg,
        None,
        "After CNF and before saturation, replaces all clauses that are definitions  of injectivity by axiomatization of inverse function.",
    ),
    OptCell::new(
        EProverOption::LiftLambdas,
        None,
        Some("lift-lambdas"),
        OptArgType::ReqArg,
        None,
        "Should the lambdas be replaced by named fuctions?",
    ),
    OptCell::new(
        EProverOption::EtaNormalize,
        None,
        Some("eta-normalize"),
        OptArgType::ReqArg,
        None,
        "Which form of eta normalization to perform?",
    ),
    OptCell::new(
        EProverOption::HoOrderKind,
        None,
        Some("ho-order-kind"),
        OptArgType::ReqArg,
        None,
        "Do we use simple LFHO order or a more advanced Boolean free lambda-KBO?",
    ),
    OptCell::new(
        EProverOption::CnfLambdaToForall,
        None,
        Some("cnf-lambda-to-forall"),
        OptArgType::ReqArg,
        None,
        "Do we turn equations of the form ^X.s (!)= ^X.t into (?)!X. s (!)= t ?",
    ),
    OptCell::new(
        EProverOption::KboLambdaWeight,
        None,
        Some("kbo-lam-weight"),
        OptArgType::ReqArg,
        None,
        "Weight of lambda symbol in KBO.",
    ),
    OptCell::new(
        EProverOption::KboDbWeight,
        None,
        Some("kbo-db-weight"),
        OptArgType::ReqArg,
        None,
        "Weight of DB var in KBO.",
    ),
    OptCell::new(
        EProverOption::EliminateLeibnizEq,
        None,
        Some("eliminate-leibniz-eq"),
        OptArgType::ReqArg,
        None,
        "Maximal proof depth of the clause on which Leibniz equality elimination should be applied; -1 disaables Leibniz equality elimination altogether",
    ),
    OptCell::new(
        EProverOption::UnrollFormulasOnly,
        None,
        Some("unroll-formulas-only"),
        OptArgType::ReqArg,
        None,
        "Set to true if you want only formulas to be recognized as definitions during CNF. Default is true.",
    ),
    OptCell::new(
        EProverOption::PrimEnumMode,
        None,
        Some("prim-enum-mode"),
        OptArgType::ReqArg,
        None,
        "Choose the mode of primitive enumeration ",
    ),
    OptCell::new(
        EProverOption::PrimEnumMaxDepth,
        None,
        Some("prim-enum-max-depth"),
        OptArgType::ReqArg,
        None,
        "Maximal proof depth of a clause on which primitive enumeration is applied. -1 disables primitive enumeration",
    ),
    OptCell::new(
        EProverOption::InstChoiceMaxDepth,
        None,
        Some("inst-choice-max-depth"),
        OptArgType::ReqArg,
        None,
        "Maximal proof depth of a clause which is going to be scanned for occurrences of defined choice symbol -1 disables scanning for choice symbols",
    ),
    OptCell::new(
        EProverOption::LocalRw,
        None,
        Some("local-rw"),
        OptArgType::ReqArg,
        None,
        "Enable/disable local rewriting: if the clause is of the form s != t |  C, where s > t, rewrite all occurrences of s with t in C.",
    ),
    OptCell::new(
        EProverOption::PruneArgs,
        None,
        Some("prune-args"),
        OptArgType::ReqArg,
        None,
        "Enable/disable pruning arguments of applied variables.",
    ),
    OptCell::new(
        EProverOption::FuncProjLimit,
        None,
        Some("func-proj-limit"),
        OptArgType::ReqArg,
        None,
        "Maximal number of functional projections",
    ),
    OptCell::new(
        EProverOption::ImitLimit,
        None,
        Some("imit-limit"),
        OptArgType::ReqArg,
        None,
        "Maximal number of imitations",
    ),
    OptCell::new(
        EProverOption::IdentLimit,
        None,
        Some("ident-limit"),
        OptArgType::ReqArg,
        None,
        "Maximal number of identifications",
    ),
    OptCell::new(
        EProverOption::ElimLimit,
        None,
        Some("elim-limit"),
        OptArgType::ReqArg,
        None,
        "Maximal number of eliminations",
    ),
    OptCell::new(
        EProverOption::UnifMode,
        None,
        Some("unif-mode"),
        OptArgType::ReqArg,
        None,
        "Set the mode of unification: either single or multi.",
    ),
    OptCell::new(
        EProverOption::PatternOracle,
        None,
        Some("pattern-oracle"),
        OptArgType::ReqArg,
        None,
        "Turn the pattern oracle on or off.",
    ),
    OptCell::new(
        EProverOption::FixpointOracle,
        None,
        Some("fixpoint-oracle"),
        OptArgType::ReqArg,
        None,
        "Turn the pattern oracle on or off.",
    ),
    OptCell::new(
        EProverOption::MaxUnifiers,
        None,
        Some("max-unifiers"),
        OptArgType::ReqArg,
        None,
        "Maximal number of imitations",
    ),
    OptCell::new(
        EProverOption::MaxUnifSteps,
        None,
        Some("max-unif-steps"),
        OptArgType::ReqArg,
        None,
        "Maximal number of variable bindings that can be done in one single call to copmuting the next unifier.",
    ),
    OptCell::new(
        EProverOption::ClassificationTimeoutPortion,
        None,
        Some("classification-timeout-portion"),
        OptArgType::ReqArg,
        None,
        "Which percentage (from 1 to 99) of the total CPU time will be devoted to problem classification?",
    ),
    OptCell::new(
        EProverOption::PreinstantiateInduction,
        None,
        Some("preinstantiate-induction"),
        OptArgType::ReqArg,
        None,
        "Abstract unit clauses coming from conjecture and use the abstractions to instantiate clauses that look like the ones coming from induction axioms.",
    ),
    OptCell::new(
        EProverOption::Bce,
        None,
        Some("bce"),
        OptArgType::ReqArg,
        None,
        "Turn blocked clause elimination on or off",
    ),
    OptCell::new(
        EProverOption::BceMaxOccs,
        None,
        Some("bce-max-occs"),
        OptArgType::ReqArg,
        None,
        "Stop tracking symbol after it occurs in <arg> clauses Set <arg> to -1 disable this limit",
    ),
    OptCell::new(
        EProverOption::PredElim,
        None,
        Some("pred-elim"),
        OptArgType::ReqArg,
        None,
        "Turn predicate elimination on or off",
    ),
    OptCell::new(
        EProverOption::PredElimMaxOccs,
        None,
        Some("pred-elim-max-occs"),
        OptArgType::ReqArg,
        None,
        "Stop tracking symbol after it occurs in <arg> clauses Set <arg> to -1 disable this limit",
    ),
    OptCell::new(
        EProverOption::PredElimTolerance,
        None,
        Some("pred-elim-tolerance"),
        OptArgType::ReqArg,
        None,
        "Tolerance for predicate elimination measures.",
    ),
    OptCell::new(
        EProverOption::PredElimRecognizeGates,
        None,
        Some("pred-elim-recognize-gates"),
        OptArgType::ReqArg,
        None,
        "Turn gate recognition for predicate elimination on or off",
    ),
    OptCell::new(
        EProverOption::PredElimForceMuDecrease,
        None,
        Some("pred-elim-force-mu-decrease"),
        OptArgType::ReqArg,
        None,
        "Require that the square number of distinct free variables decreases when doing predicate elimination. Helps avoid creating huge clauses.",
    ),
    OptCell::new(
        EProverOption::PredElimIgnoreConjSyms,
        None,
        Some("pred-elim-ignore-conj-syms"),
        OptArgType::ReqArg,
        None,
        "Disable eliminating symbols that occur in the conjecture.",
    ),
];

#[cfg(test)]
mod tests {
    use super::EPROVER_OPTIONS;
    use crate::inout::commandline::OptArgType;

    const C_E_OPTIONS_H: &str = include_str!("../../eprover/PROVER/e_options.h");

    #[test]
    fn rust_option_table_matches_c_long_option_surface() {
        let rust_long_options = EPROVER_OPTIONS
            .iter()
            .filter_map(|option| option.longopt)
            .collect::<Vec<_>>();
        let c_long_options = c_long_options();

        assert_has_no_duplicates("Rust", &rust_long_options);
        assert_has_no_duplicates("C", &c_long_options);
        assert_eq!(rust_long_options, c_long_options);
    }

    #[test]
    fn rust_option_table_matches_c_short_option_surface() {
        let rust_short_options = EPROVER_OPTIONS
            .iter()
            .filter_map(|option| option.shortopt)
            .collect::<Vec<_>>();
        let c_short_options = c_short_options();

        assert_has_no_duplicates("Rust", &rust_short_options);
        assert_has_no_duplicates("C", &c_short_options);
        assert_eq!(rust_short_options, c_short_options);
    }

    #[test]
    fn rust_option_table_matches_c_argument_surface() {
        let rust_argument_surface = rust_argument_surface();
        let c_argument_surface = c_argument_surface();

        assert_eq!(rust_argument_surface, c_argument_surface);
    }

    #[test]
    fn rust_option_table_matches_c_help_prose() {
        let rust_help = EPROVER_OPTIONS
            .iter()
            .map(|option| (option.longopt, option.desc.to_owned()))
            .collect::<Vec<_>>();
        let c_help = c_help_surface();

        assert_eq!(rust_help.len(), c_help.len());
        for ((rust_name, rust_description), (c_name, c_description)) in
            rust_help.iter().zip(&c_help)
        {
            assert_eq!(rust_name, c_name);
            if *rust_name == Some("tstp-in") {
                assert!(rust_description.contains("Umlaut"));
                assert!(rust_description.contains("TPTP 8.2.0"));
            } else if matches!(
                *rust_name,
                Some(
                    "error-on-empty"
                        | "lop-in"
                        | "soft-cpu-limit"
                        | "assume-completeness"
                        | "order-weights"
                        | "prefer-general-demodulators"
                )
            ) {
                assert_eq!(
                    rust_description,
                    &c_description.replace(" E ", " Umlaut "),
                    "option {rust_name:?}"
                );
            } else {
                assert_eq!(rust_description, c_description, "option {rust_name:?}");
            }
        }
    }

    fn assert_has_no_duplicates<T>(table_name: &str, options: &[T])
    where
        T: Clone + Ord + std::fmt::Debug,
    {
        let mut sorted_options = options.to_vec();
        sorted_options.sort_unstable();

        for adjacent_options in sorted_options.windows(2) {
            assert_ne!(
                adjacent_options[0], adjacent_options[1],
                "{table_name} option table has duplicate option {:?}",
                adjacent_options[0]
            );
        }
    }

    #[must_use]
    fn c_long_options() -> Vec<&'static str> {
        let table = c_option_table_body();
        option_entries(table)
            .into_iter()
            .filter_map(long_option_from_entry)
            .collect()
    }

    #[must_use]
    fn c_short_options() -> Vec<char> {
        let table = c_option_table_body();
        option_entries(table)
            .into_iter()
            .filter_map(short_option_from_entry)
            .collect()
    }

    #[must_use]
    fn rust_argument_surface() -> Vec<ArgumentSurface> {
        EPROVER_OPTIONS
            .iter()
            .map(|option| ArgumentSurface {
                short_option: option.shortopt,
                long_option: option.longopt,
                argument_type: arg_type_name(option.arg_type),
                argument_default: option.arg_default,
            })
            .collect()
    }

    #[must_use]
    fn c_argument_surface() -> Vec<ArgumentSurface> {
        let table = c_option_table_body();
        option_entries(table)
            .into_iter()
            .filter_map(argument_surface_from_entry)
            .collect()
    }

    #[must_use]
    fn c_help_surface() -> Vec<(Option<&'static str>, String)> {
        let table = c_option_table_body();
        option_entries(table)
            .into_iter()
            .filter_map(|entry| {
                let fields = split_c_fields(entry);
                let option_code = fields.first().expect("option entry must have a code");
                if *option_code == "OPT_NOOPT" {
                    return None;
                }
                let long_option = fields.get(2).expect("option entry must have a long name");
                let description = fields.get(5).expect("option entry must have help prose");
                Some((
                    c_string_literal(long_option),
                    c_string_expression(description),
                ))
            })
            .collect()
    }

    #[must_use]
    fn c_option_table_body() -> &'static str {
        let table_start = C_E_OPTIONS_H
            .find("OptCell opts[]")
            .expect("C options table must be present");
        let table = &C_E_OPTIONS_H[table_start..];
        let body_start = table.find('{').expect("C options table must have a body") + 1;
        let body_end = table.find("\n};").expect("C options table must be closed");
        &table[body_start..body_end]
    }

    #[must_use]
    fn option_entries(table: &'static str) -> Vec<&'static str> {
        let mut entries = Vec::new();
        let mut entry_start = None;
        let mut brace_depth = 0_usize;
        let mut in_string = false;
        let mut in_char = false;
        let mut escaped = false;

        for (index, character) in table.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if in_string {
                match character {
                    '\\' => escaped = true,
                    '"' => in_string = false,
                    _ => {}
                }
                continue;
            }
            if in_char {
                match character {
                    '\\' => escaped = true,
                    '\'' => in_char = false,
                    _ => {}
                }
                continue;
            }

            match character {
                '"' => in_string = true,
                '\'' => in_char = true,
                '{' => {
                    if brace_depth == 0 {
                        entry_start = Some(index + 1);
                    }
                    brace_depth += 1;
                }
                '}' => {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        let start = entry_start.expect("entry start must be set");
                        entries.push(&table[start..index]);
                        entry_start = None;
                    }
                }
                _ => {}
            }
        }

        entries
    }

    #[must_use]
    fn long_option_from_entry(entry: &'static str) -> Option<&'static str> {
        let fields = split_c_fields(entry);
        let option_code = fields.first().expect("option entry must have a code");
        if *option_code == "OPT_NOOPT" {
            return None;
        }
        let long_field = fields.get(2).expect("option entry must have a long name");
        c_string_literal(long_field)
    }

    #[must_use]
    fn short_option_from_entry(entry: &'static str) -> Option<char> {
        let fields = split_c_fields(entry);
        let option_code = fields.first().expect("option entry must have a code");
        if *option_code == "OPT_NOOPT" {
            return None;
        }
        let short_field = fields.get(1).expect("option entry must have a short name");
        c_char_literal(short_field)
    }

    #[must_use]
    fn argument_surface_from_entry(entry: &'static str) -> Option<ArgumentSurface> {
        let fields = split_c_fields(entry);
        let option_code = fields.first().expect("option entry must have a code");
        if *option_code == "OPT_NOOPT" {
            return None;
        }

        let short_field = fields.get(1).expect("option entry must have a short name");
        let long_field = fields.get(2).expect("option entry must have a long name");
        let argument_type = fields
            .get(3)
            .copied()
            .expect("option entry must have an argument type");
        let default_field = fields
            .get(4)
            .expect("option entry must have an argument default");

        Some(ArgumentSurface {
            short_option: c_char_literal(short_field),
            long_option: c_string_literal(long_field),
            argument_type,
            argument_default: c_default_literal(default_field),
        })
    }

    #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct ArgumentSurface {
        short_option: Option<char>,
        long_option: Option<&'static str>,
        argument_type: &'static str,
        argument_default: Option<&'static str>,
    }

    #[must_use]
    const fn arg_type_name(arg_type: OptArgType) -> &'static str {
        match arg_type {
            OptArgType::NoArg => "NoArg",
            OptArgType::OptArg => "OptArg",
            OptArgType::ReqArg => "ReqArg",
        }
    }

    #[must_use]
    fn split_c_fields(entry: &'static str) -> Vec<&'static str> {
        let mut fields = Vec::new();
        let mut field_start = 0_usize;
        let mut in_string = false;
        let mut in_char = false;
        let mut escaped = false;

        for (index, character) in entry.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if in_string {
                match character {
                    '\\' => escaped = true,
                    '"' => in_string = false,
                    _ => {}
                }
                continue;
            }
            if in_char {
                match character {
                    '\\' => escaped = true,
                    '\'' => in_char = false,
                    _ => {}
                }
                continue;
            }

            match character {
                '"' => in_string = true,
                '\'' => in_char = true,
                ',' => {
                    fields.push(entry[field_start..index].trim());
                    field_start = index + 1;
                }
                _ => {}
            }
        }

        fields.push(entry[field_start..].trim());
        fields
    }

    #[must_use]
    fn c_string_literal(field: &'static str) -> Option<&'static str> {
        let trimmed = field.trim();
        if trimmed == "NULL" {
            return None;
        }
        let literal_body = trimmed
            .strip_prefix('"')
            .expect("long option field must be a C string or NULL");
        let literal_end = literal_body
            .find('"')
            .expect("long option string must be terminated");
        Some(&literal_body[..literal_end])
    }

    #[must_use]
    fn c_default_literal(field: &'static str) -> Option<&'static str> {
        match field.trim() {
            "DEFAULT_OUTPUT_DESCRIPTOR" => Some("eigEIG"),
            "DEFAULT_FILTER_DESCRIPTOR" => Some("Fc"),
            "WATCHLIST_INLINE_QSTRING" => Some("'Use inline watchlist type'"),
            "TFORM_RENAME_LIMIT_STR" => Some("24"),
            "TFORM_MINISCOPE_LIMIT_STR" => Some("2147483648"),
            _ => c_string_literal(field),
        }
    }

    #[must_use]
    fn c_string_expression(field: &str) -> String {
        let mut result = String::new();
        let bytes = field.as_bytes();
        let mut index = 0_usize;

        while index < bytes.len() {
            match bytes[index] {
                byte if byte.is_ascii_whitespace() => index += 1,
                b'\\'
                    if bytes.get(index + 1) == Some(&b'\r')
                        && bytes.get(index + 2) == Some(&b'\n') =>
                {
                    index += 3;
                }
                b'\\' if bytes.get(index + 1) == Some(&b'\n') => index += 2,
                b'"' => {
                    index += 1;
                    while index < bytes.len() && bytes[index] != b'"' {
                        if bytes[index] == b'\\' {
                            index += 1;
                            let escaped = *bytes
                                .get(index)
                                .expect("C help string escape must have a value");
                            result.push(match escaped {
                                b'\\' => '\\',
                                b'"' => '"',
                                b'n' => '\n',
                                b'r' => '\r',
                                b't' => '\t',
                                b'\'' => '\'',
                                _ => panic!("unsupported C help string escape {escaped:?}"),
                            });
                        } else {
                            result.push(char::from(bytes[index]));
                        }
                        index += 1;
                    }
                    assert_eq!(bytes.get(index), Some(&b'"'), "unterminated C help string");
                    index += 1;
                }
                _ if field[index..].starts_with("NAME") => {
                    result.push_str("umlaut");
                    index += "NAME".len();
                }
                _ if field[index..].starts_with("WATCHLIST_INLINE_QSTRING") => {
                    result.push_str("'Use inline watchlist type'");
                    index += "WATCHLIST_INLINE_QSTRING".len();
                }
                _ => panic!("unsupported C help expression suffix {:?}", &field[index..]),
            }
        }
        result
    }

    #[must_use]
    fn c_char_literal(field: &str) -> Option<char> {
        let trimmed = field.trim();
        if trimmed == r"'\0'" {
            return None;
        }
        let literal_body = trimmed
            .strip_prefix('\'')
            .and_then(|body| body.strip_suffix('\''))
            .expect("short option field must be a C character literal");
        let mut characters = literal_body.chars();
        let short_option = characters
            .next()
            .expect("short option literal must not be empty");
        assert!(
            characters.next().is_none(),
            "short option literal must contain one character"
        );
        Some(short_option)
    }
}
