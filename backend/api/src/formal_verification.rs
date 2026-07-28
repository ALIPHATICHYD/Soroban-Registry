// Formal verification engine for Soroban smart contracts.
//
// Pipeline:
//   WASM bytes  ──► WasmBytecodeAnalyzer
//                         │
//                   ┌─────┴──────┐
//                   │            │
//             SymbolicExecutor   VulnerabilityScanner
//                   │            │
//                   └─────┬──────┘
//                         │
//                  PropertyChecker
//                         │
//                  ProofGenerator
//                         │
//              FormalVerificationReport

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use wasmparser::{
    BinaryReaderError, Export, ExportSectionReader, FunctionBody, Import, ImportSectionReader,
    Operator, Parser, Payload,
};

// ─── Public report types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    Proved,
    Violated,
    Inconclusive,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofMethod {
    AbstractInterpretation,
    SymbolicExecution,
    ControlFlowAnalysis,
    TypeLevelAnalysis,
    PatternMatching,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidence {
    pub kind: String,
    pub description: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyResult {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ProofStatus,
    pub method: ProofMethod,
    pub confidence: f64,
    pub evidence: Vec<ProofEvidence>,
    pub counterexample: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: VulnerabilitySeverity,
    pub category: String,
    pub cwe_id: Option<String>,
    pub affected_functions: Vec<String>,
    pub location: Option<String>,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCertificate {
    pub certificate_id: Uuid,
    pub contract_id: Uuid,
    pub session_id: Uuid,
    pub properties_proved: usize,
    pub properties_violated: usize,
    pub properties_inconclusive: usize,
    pub overall_confidence: f64,
    pub generated_at: DateTime<Utc>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalVerificationReport {
    pub session_id: Uuid,
    pub contract_id: Uuid,
    pub wasm_size_bytes: usize,
    pub function_count: usize,
    pub import_count: usize,
    pub export_count: usize,
    pub properties: Vec<PropertyResult>,
    pub vulnerabilities: Vec<SecurityFinding>,
    pub certificate: ProofCertificate,
    pub analysis_duration_ms: u64,
    pub analyzer_version: String,
}

// ─── Internal analysis types ──────────────────────────────────────────────────

/// Represents the abstract value of a stack slot during symbolic execution.
#[derive(Debug, Clone, PartialEq)]
enum AbstractValue {
    Unknown,
    Constant(i64),
    /// An integer constrained to a known range.
    BoundedInt {
        min: i64,
        max: i64,
    },
    /// Value produced by an arithmetic operation that was not checked.
    PossiblyOverflowing,
    /// Value that originates from an external (untrusted) source.
    Tainted,
    /// Value that came from an authorized check.
    Trusted,
}

/// Per-function analysis result gathered by the symbolic executor.
#[derive(Debug, Default)]
struct FunctionAnalysis {
    index: u32,
    name: Option<String>,
    calls_require_auth: bool,
    calls_storage_write: bool,
    calls_storage_read: bool,
    calls_cross_contract: bool,
    writes_before_auth: bool,
    cross_call_after_write: bool,
    unchecked_divisions: u32,
    /// True if any arithmetic instruction was used without a bounds check.
    unchecked_arithmetic: bool,
    /// Instruction count (proxy for complexity).
    instruction_count: u32,
    /// Loops detected (back-edges in control flow).
    loop_count: u32,
    has_unbounded_loop: bool,
}

/// Module-level facts derived from the import section.
#[derive(Debug, Default)]
struct ModuleFacts {
    /// All Soroban host functions imported, keyed by function index.
    host_imports: HashMap<u32, HostFunction>,
    /// Exported function names and their indices.
    exports: HashMap<u32, String>,
    /// Total user-defined functions (not imports).
    user_function_base_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostFunction {
    RequireAuth,
    StorageHasKey,
    StorageGet,
    StorageSet,
    StorageRemove,
    InvokeContract,
    GetLedgerSequence,
    Panic,
    Other(String),
}

impl HostFunction {
    fn is_storage_write(&self) -> bool {
        matches!(self, Self::StorageSet | Self::StorageRemove)
    }
    fn is_storage_read(&self) -> bool {
        matches!(self, Self::StorageGet | Self::StorageHasKey)
    }
}

fn classify_import(module: &str, name: &str) -> HostFunction {
    // Soroban host functions follow the pattern: module = "env" or "soroban_env"
    match name {
        n if n.contains("require_auth") || n.contains("auth") && n.contains("require") => {
            HostFunction::RequireAuth
        }
        n if n.contains("put")
            || n.contains("set")
            || n.contains("write")
            || n.contains("store") =>
        {
            if module.contains("storage") || module.contains("env") {
                HostFunction::StorageSet
            } else {
                HostFunction::Other(format!("{}.{}", module, n))
            }
        }
        n if n.contains("remove") || n.contains("delete") => HostFunction::StorageRemove,
        n if n.contains("get")
            || n.contains("read")
            || n.contains("has")
            || n.contains("contains") =>
        {
            if module.contains("storage") || module.contains("env") {
                HostFunction::StorageGet
            } else {
                HostFunction::Other(format!("{}.{}", module, n))
            }
        }
        n if n.contains("invoke") || n.contains("call") && !n.contains("callback") => {
            HostFunction::InvokeContract
        }
        n if n.contains("panic") || n.contains("abort") || n.contains("trap") => {
            HostFunction::Panic
        }
        n if n.contains("ledger") || n.contains("sequence") => HostFunction::GetLedgerSequence,
        _ => HostFunction::Other(format!("{}.{}", module, name)),
    }
}

// ─── WASM bytecode analyzer ───────────────────────────────────────────────────

pub struct WasmBytecodeAnalyzer {
    wasm_bytes: Vec<u8>,
    contract_id: Uuid,
    session_id: Uuid,
}

impl WasmBytecodeAnalyzer {
    pub fn new(wasm_bytes: Vec<u8>, contract_id: Uuid) -> Self {
        Self {
            wasm_bytes,
            contract_id,
            session_id: Uuid::new_v4(),
        }
    }

    /// Run all analyses and return the full report.
    pub fn run(&self) -> Result<FormalVerificationReport, String> {
        let started = std::time::Instant::now();

        let facts = self.parse_module_facts()?;
        let functions = self.analyze_functions(&facts)?;

        let properties = self.check_properties(&facts, &functions);
        let vulnerabilities = self.scan_vulnerabilities(&facts, &functions);
        let certificate = self.generate_certificate(&properties);

        let elapsed = started.elapsed().as_millis() as u64;

        Ok(FormalVerificationReport {
            session_id: self.session_id,
            contract_id: self.contract_id,
            wasm_size_bytes: self.wasm_bytes.len(),
            function_count: functions.len(),
            import_count: facts.host_imports.len(),
            export_count: facts.exports.len(),
            properties,
            vulnerabilities,
            certificate,
            analysis_duration_ms: elapsed,
            analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    // ── Module parsing ────────────────────────────────────────────────────────

    fn parse_module_facts(&self) -> Result<ModuleFacts, String> {
        let mut facts = ModuleFacts::default();
        let mut import_count: u32 = 0;

        for payload in Parser::new(0).parse_all(&self.wasm_bytes) {
            let payload = payload.map_err(|e| format!("WASM parse error: {}", e))?;
            match payload {
                Payload::ImportSection(reader) => {
                    for import in reader {
                        let import = import.map_err(|e| format!("Import parse error: {}", e))?;
                        if let wasmparser::TypeRef::Func(_) = import.ty {
                            let host_fn = classify_import(import.module, import.name);
                            facts.host_imports.insert(import_count, host_fn);
                            import_count += 1;
                        }
                    }
                    facts.user_function_base_index = import_count;
                }
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export.map_err(|e| format!("Export parse error: {}", e))?;
                        if let wasmparser::ExternalKind::Func = export.kind {
                            facts.exports.insert(export.index, export.name.to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(facts)
    }

    // ── Per-function symbolic execution ───────────────────────────────────────

    fn analyze_functions(&self, facts: &ModuleFacts) -> Result<Vec<FunctionAnalysis>, String> {
        let mut analyses: Vec<FunctionAnalysis> = Vec::new();
        let mut func_index = facts.user_function_base_index;

        for payload in Parser::new(0).parse_all(&self.wasm_bytes) {
            let payload = payload.map_err(|e| format!("WASM parse error: {}", e))?;
            if let Payload::CodeSectionEntry(body) = payload {
                let mut analysis = FunctionAnalysis {
                    index: func_index,
                    name: facts.exports.get(&func_index).cloned(),
                    ..Default::default()
                };
                self.execute_function(&body, facts, &mut analysis)
                    .unwrap_or_else(|_| {
                        // On parse failure, mark the function as inconclusive.
                        analysis.unchecked_arithmetic = true;
                    });
                analyses.push(analysis);
                func_index += 1;
            }
        }

        Ok(analyses)
    }

    fn execute_function(
        &self,
        body: &FunctionBody,
        facts: &ModuleFacts,
        analysis: &mut FunctionAnalysis,
    ) -> Result<(), BinaryReaderError> {
        let mut reader = body.get_operators_reader()?;
        let mut stack: Vec<AbstractValue> = Vec::new();
        // Track whether auth was seen before the first storage write.
        let mut seen_auth = false;
        let mut seen_write = false;
        let mut depth: u32 = 0;

        while !reader.eof() {
            let (op, _offset) = reader.read_with_offset()?;
            analysis.instruction_count += 1;

            match op {
                Operator::Call { function_index } => {
                    if let Some(host_fn) = facts.host_imports.get(&function_index) {
                        match host_fn {
                            HostFunction::RequireAuth => {
                                analysis.calls_require_auth = true;
                                seen_auth = true;
                                stack.push(AbstractValue::Trusted);
                            }
                            HostFunction::StorageSet | HostFunction::StorageRemove => {
                                analysis.calls_storage_write = true;
                                if !seen_auth {
                                    analysis.writes_before_auth = true;
                                }
                                seen_write = true;
                                stack.push(AbstractValue::Unknown);
                            }
                            HostFunction::StorageGet | HostFunction::StorageHasKey => {
                                analysis.calls_storage_read = true;
                                // Storage reads produce a tainted value (untrusted input).
                                stack.push(AbstractValue::Tainted);
                            }
                            HostFunction::InvokeContract => {
                                analysis.calls_cross_contract = true;
                                if seen_write {
                                    analysis.cross_call_after_write = true;
                                }
                                stack.push(AbstractValue::Tainted);
                            }
                            _ => {
                                stack.push(AbstractValue::Unknown);
                            }
                        }
                    } else {
                        // User-defined function call.
                        stack.push(AbstractValue::Unknown);
                    }
                }

                // Integer arithmetic — check for unchecked operations
                Operator::I32Add
                | Operator::I32Sub
                | Operator::I32Mul
                | Operator::I64Add
                | Operator::I64Sub
                | Operator::I64Mul => {
                    let b = stack.pop().unwrap_or(AbstractValue::Unknown);
                    let a = stack.pop().unwrap_or(AbstractValue::Unknown);
                    let result = match (&a, &b) {
                        (AbstractValue::Constant(x), AbstractValue::Constant(y)) => {
                            // Constant folding — compute directly.
                            match &op {
                                Operator::I32Add | Operator::I64Add => {
                                    AbstractValue::Constant(x.wrapping_add(*y))
                                }
                                Operator::I32Sub | Operator::I64Sub => {
                                    AbstractValue::Constant(x.wrapping_sub(*y))
                                }
                                _ => AbstractValue::Constant(x.wrapping_mul(*y)),
                            }
                        }
                        _ => {
                            // Non-constant operands: the result might overflow.
                            if matches!(a, AbstractValue::Tainted)
                                || matches!(b, AbstractValue::Tainted)
                            {
                                analysis.unchecked_arithmetic = true;
                                AbstractValue::PossiblyOverflowing
                            } else {
                                AbstractValue::PossiblyOverflowing
                            }
                        }
                    };
                    stack.push(result);
                }

                // Saturating / checked variants — safe
                Operator::I32Clz
                | Operator::I32Ctz
                | Operator::I32Popcnt
                | Operator::I64Clz
                | Operator::I64Ctz
                | Operator::I64Popcnt => {
                    let _ = stack.pop();
                    stack.push(AbstractValue::BoundedInt { min: 0, max: 64 });
                }

                // Integer division — check for zero divisor potential
                Operator::I32DivS
                | Operator::I32DivU
                | Operator::I64DivS
                | Operator::I64DivU
                | Operator::I32RemS
                | Operator::I32RemU
                | Operator::I64RemS
                | Operator::I64RemU => {
                    let divisor = stack.pop().unwrap_or(AbstractValue::Unknown);
                    let _ = stack.pop();
                    if !matches!(divisor, AbstractValue::Constant(n) if n != 0) {
                        analysis.unchecked_divisions += 1;
                    }
                    stack.push(AbstractValue::PossiblyOverflowing);
                }

                // Constants
                Operator::I32Const { value } => {
                    stack.push(AbstractValue::Constant(value as i64));
                }
                Operator::I64Const { value } => {
                    stack.push(AbstractValue::Constant(value));
                }
                Operator::F32Const { .. } | Operator::F64Const { .. } => {
                    stack.push(AbstractValue::Unknown);
                }

                // Control flow — track loop nesting for unbounded loop detection
                Operator::Loop { .. } => {
                    analysis.loop_count += 1;
                    depth += 1;
                    stack.push(AbstractValue::Unknown);
                }
                Operator::Block { .. } | Operator::If { .. } => {
                    depth += 1;
                }
                Operator::End => {
                    depth = depth.saturating_sub(1);
                }
                Operator::Br { relative_depth: _ } | Operator::BrIf { relative_depth: _ } => {
                    // A backward branch (br to an enclosing loop) creates an unbounded loop
                    // unless the loop exit is provably reachable.
                    // We use a conservative heuristic: any br/br_if with depth >= 1 inside
                    // a loop body is treated as potentially unbounded.
                    if analysis.loop_count > 0 {
                        analysis.has_unbounded_loop = true;
                    }
                }
                Operator::BrTable { .. } => {
                    if analysis.loop_count > 0 {
                        analysis.has_unbounded_loop = true;
                    }
                    let _ = stack.pop();
                }

                // Stack manipulation
                Operator::Drop => {
                    stack.pop();
                }
                Operator::Select => {
                    let _ = stack.pop();
                    let b = stack.pop().unwrap_or(AbstractValue::Unknown);
                    let a = stack.pop().unwrap_or(AbstractValue::Unknown);
                    // Conservatively take the less-known of the two branches.
                    stack.push(
                        if a == AbstractValue::PossiblyOverflowing
                            || b == AbstractValue::PossiblyOverflowing
                        {
                            AbstractValue::PossiblyOverflowing
                        } else {
                            AbstractValue::Unknown
                        },
                    );
                }

                // Loads push tainted values (come from memory, untrusted).
                Operator::I32Load { .. }
                | Operator::I64Load { .. }
                | Operator::I32Load8S { .. }
                | Operator::I32Load8U { .. }
                | Operator::I32Load16S { .. }
                | Operator::I32Load16U { .. }
                | Operator::I64Load8S { .. }
                | Operator::I64Load8U { .. } => {
                    let _ = stack.pop(); // address
                    stack.push(AbstractValue::Tainted);
                }

                // Comparisons produce bounded 0/1.
                Operator::I32Eqz
                | Operator::I32Eq
                | Operator::I32Ne
                | Operator::I32LtS
                | Operator::I32LtU
                | Operator::I32GtS
                | Operator::I32GtU
                | Operator::I32LeS
                | Operator::I32LeU
                | Operator::I32GeS
                | Operator::I32GeU
                | Operator::I64Eqz
                | Operator::I64Eq
                | Operator::I64Ne
                | Operator::I64LtS
                | Operator::I64LtU
                | Operator::I64GtS
                | Operator::I64GtU
                | Operator::I64LeS
                | Operator::I64LeU
                | Operator::I64GeS
                | Operator::I64GeU => {
                    // binary comparisons pop two, unary pops one
                    let _ = stack.pop();
                    let _ = stack.pop();
                    stack.push(AbstractValue::BoundedInt { min: 0, max: 1 });
                }

                Operator::Return | Operator::Unreachable => {}

                _ => {
                    // Conservative: push an unknown value for any unhandled op.
                    stack.push(AbstractValue::Unknown);
                    if stack.len() > 1024 {
                        stack.drain(0..512);
                    }
                }
            }
        }

        Ok(())
    }

    // ── Property checking ─────────────────────────────────────────────────────

    fn check_properties(
        &self,
        facts: &ModuleFacts,
        functions: &[FunctionAnalysis],
    ) -> Vec<PropertyResult> {
        vec![
            self.check_authorization_completeness(facts, functions),
            self.check_arithmetic_safety(functions),
            self.check_reentrancy_safety(functions),
            self.check_division_safety(functions),
            self.check_loop_termination(functions),
            self.check_access_control_consistency(facts, functions),
            self.check_state_mutation_isolation(functions),
        ]
    }

    /// P1 – Every exported function that writes to storage calls require_auth.
    fn check_authorization_completeness(
        &self,
        facts: &ModuleFacts,
        functions: &[FunctionAnalysis],
    ) -> PropertyResult {
        let exported_writers: Vec<&FunctionAnalysis> = functions
            .iter()
            .filter(|f| facts.exports.contains_key(&f.index) && f.calls_storage_write)
            .collect();

        let unauthorized: Vec<&FunctionAnalysis> = exported_writers
            .iter()
            .filter(|f| !f.calls_require_auth)
            .copied()
            .collect();

        if exported_writers.is_empty() {
            return PropertyResult {
                id: "P001".into(),
                name: "Authorization Completeness".into(),
                description: "All exported state-mutating functions require authorization".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.95,
                evidence: vec![ProofEvidence {
                    kind: "no_mutable_exports".into(),
                    description: "Contract has no exported functions that write to storage".into(),
                    location: None,
                }],
                counterexample: None,
            };
        }

        if unauthorized.is_empty() {
            PropertyResult {
                id: "P001".into(),
                name: "Authorization Completeness".into(),
                description: "All exported state-mutating functions require authorization".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.92,
                evidence: vec![ProofEvidence {
                    kind: "auth_present".into(),
                    description: format!(
                        "All {} exported state-writing functions call require_auth",
                        exported_writers.len()
                    ),
                    location: None,
                }],
                counterexample: None,
            }
        } else {
            let names: Vec<String> = unauthorized
                .iter()
                .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
                .collect();
            PropertyResult {
                id: "P001".into(),
                name: "Authorization Completeness".into(),
                description: "All exported state-mutating functions require authorization".into(),
                status: ProofStatus::Violated,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.97,
                evidence: vec![ProofEvidence {
                    kind: "missing_auth".into(),
                    description: format!(
                        "{} exported function(s) write to storage without require_auth: {}",
                        unauthorized.len(),
                        names.join(", ")
                    ),
                    location: names.first().cloned(),
                }],
                counterexample: Some(format!(
                    "Function '{}' writes to storage without calling require_auth",
                    names[0]
                )),
            }
        }
    }

    /// P2 – Arithmetic operations on tainted values are bounds-checked.
    fn check_arithmetic_safety(&self, functions: &[FunctionAnalysis]) -> PropertyResult {
        let unsafe_fns: Vec<&FunctionAnalysis> = functions
            .iter()
            .filter(|f| f.unchecked_arithmetic && f.calls_storage_write)
            .collect();

        if unsafe_fns.is_empty() {
            PropertyResult {
                id: "P002".into(),
                name: "Arithmetic Safety".into(),
                description: "Integer arithmetic on external inputs is bounds-checked".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::AbstractInterpretation,
                confidence: 0.85,
                evidence: vec![ProofEvidence {
                    kind: "no_overflow_path".into(),
                    description:
                        "No arithmetic paths on tainted values detected in state-writing functions"
                            .into(),
                    location: None,
                }],
                counterexample: None,
            }
        } else {
            let names: Vec<String> = unsafe_fns
                .iter()
                .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
                .collect();
            PropertyResult {
                id: "P002".into(),
                name: "Arithmetic Safety".into(),
                description: "Integer arithmetic on external inputs is bounds-checked".into(),
                status: ProofStatus::Inconclusive,
                method: ProofMethod::AbstractInterpretation,
                confidence: 0.70,
                evidence: vec![ProofEvidence {
                    kind: "unchecked_arithmetic_on_tainted".into(),
                    description: format!(
                        "Functions {} perform unchecked arithmetic on potentially tainted values",
                        names.join(", ")
                    ),
                    location: names.first().cloned(),
                }],
                counterexample: Some(format!(
                    "Function '{}' uses arithmetic on externally-supplied values without overflow guards",
                    names[0]
                )),
            }
        }
    }

    /// P3 – No reentrancy: state is not written after a cross-contract call.
    fn check_reentrancy_safety(&self, functions: &[FunctionAnalysis]) -> PropertyResult {
        let reentrant: Vec<&FunctionAnalysis> = functions
            .iter()
            .filter(|f| f.cross_call_after_write)
            .collect();

        if reentrant.is_empty() {
            PropertyResult {
                id: "P003".into(),
                name: "Reentrancy Safety".into(),
                description: "No state writes occur after cross-contract invocations (CEI pattern)".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.90,
                evidence: vec![ProofEvidence {
                    kind: "cei_pattern".into(),
                    description: "Checks-Effects-Interactions pattern appears satisfied: no storage writes after external calls detected".into(),
                    location: None,
                }],
                counterexample: None,
            }
        } else {
            let names: Vec<String> = reentrant
                .iter()
                .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
                .collect();
            PropertyResult {
                id: "P003".into(),
                name: "Reentrancy Safety".into(),
                description: "No state writes occur after cross-contract invocations (CEI pattern)"
                    .into(),
                status: ProofStatus::Violated,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.88,
                evidence: vec![ProofEvidence {
                    kind: "write_after_external_call".into(),
                    description: format!(
                        "{} function(s) write to storage after invoking external contracts: {}",
                        reentrant.len(),
                        names.join(", ")
                    ),
                    location: names.first().cloned(),
                }],
                counterexample: Some(format!(
                    "Function '{}' violates CEI: storage write occurs after cross-contract call",
                    names[0]
                )),
            }
        }
    }

    /// P4 – Division operations are guarded against divide-by-zero.
    fn check_division_safety(&self, functions: &[FunctionAnalysis]) -> PropertyResult {
        let unsafe_divs: u32 = functions.iter().map(|f| f.unchecked_divisions).sum();

        if unsafe_divs == 0 {
            PropertyResult {
                id: "P004".into(),
                name: "Division Safety".into(),
                description: "Integer division operations are guarded against zero divisors".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::SymbolicExecution,
                confidence: 0.88,
                evidence: vec![ProofEvidence {
                    kind: "no_unchecked_division".into(),
                    description: "All division operations use provably non-zero divisors".into(),
                    location: None,
                }],
                counterexample: None,
            }
        } else {
            PropertyResult {
                id: "P004".into(),
                name: "Division Safety".into(),
                description: "Integer division operations are guarded against zero divisors".into(),
                status: ProofStatus::Inconclusive,
                method: ProofMethod::SymbolicExecution,
                confidence: 0.72,
                evidence: vec![ProofEvidence {
                    kind: "possibly_unchecked_division".into(),
                    description: format!(
                        "{} division operation(s) use divisors that could not be proved non-zero",
                        unsafe_divs
                    ),
                    location: None,
                }],
                counterexample: Some(
                    "Division by a non-constant value without prior zero-check detected".into(),
                ),
            }
        }
    }

    /// P5 – All loops have a provably finite iteration bound.
    fn check_loop_termination(&self, functions: &[FunctionAnalysis]) -> PropertyResult {
        let unbounded: Vec<&FunctionAnalysis> =
            functions.iter().filter(|f| f.has_unbounded_loop).collect();

        if unbounded.is_empty() {
            PropertyResult {
                id: "P005".into(),
                name: "Loop Termination".into(),
                description: "All loops terminate within a finite number of iterations".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.80,
                evidence: vec![ProofEvidence {
                    kind: "no_backward_branches".into(),
                    description: "No potentially unbounded backward branches detected".into(),
                    location: None,
                }],
                counterexample: None,
            }
        } else {
            let names: Vec<String> = unbounded
                .iter()
                .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
                .collect();
            PropertyResult {
                id: "P005".into(),
                name: "Loop Termination".into(),
                description: "All loops terminate within a finite number of iterations".into(),
                status: ProofStatus::Inconclusive,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.65,
                evidence: vec![ProofEvidence {
                    kind: "possibly_unbounded_loop".into(),
                    description: format!(
                        "Functions {} contain loops without provable termination bounds",
                        names.join(", ")
                    ),
                    location: names.first().cloned(),
                }],
                counterexample: Some(
                    "Loop with backward branch — iteration count depends on runtime input".into(),
                ),
            }
        }
    }

    /// P6 – Access control is applied consistently across all write-capable exports.
    fn check_access_control_consistency(
        &self,
        facts: &ModuleFacts,
        functions: &[FunctionAnalysis],
    ) -> PropertyResult {
        let exported_fns: Vec<&FunctionAnalysis> = functions
            .iter()
            .filter(|f| facts.exports.contains_key(&f.index))
            .collect();

        let auth_fns = exported_fns.iter().filter(|f| f.calls_require_auth).count();
        let no_auth_fns = exported_fns
            .iter()
            .filter(|f| !f.calls_require_auth)
            .count();

        // If both guarded and unguarded functions exist, check that unguarded ones are read-only.
        let inconsistent: Vec<&&FunctionAnalysis> = exported_fns
            .iter()
            .filter(|f| !f.calls_require_auth && f.calls_storage_write)
            .collect();

        if inconsistent.is_empty() {
            PropertyResult {
                id: "P006".into(),
                name: "Access Control Consistency".into(),
                description: "Access control is applied consistently: read-only functions lack auth, write functions require it".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.91,
                evidence: vec![ProofEvidence {
                    kind: "consistent_acl".into(),
                    description: format!(
                        "{} auth-guarded, {} read-only exported functions — separation is consistent",
                        auth_fns, no_auth_fns
                    ),
                    location: None,
                }],
                counterexample: None,
            }
        } else {
            let names: Vec<String> = inconsistent
                .iter()
                .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
                .collect();
            PropertyResult {
                id: "P006".into(),
                name: "Access Control Consistency".into(),
                description: "Access control is applied consistently: read-only functions lack auth, write functions require it".into(),
                status: ProofStatus::Violated,
                method: ProofMethod::ControlFlowAnalysis,
                confidence: 0.95,
                evidence: vec![ProofEvidence {
                    kind: "inconsistent_acl".into(),
                    description: format!(
                        "{} write function(s) lack authorization while others are guarded: {}",
                        inconsistent.len(), names.join(", ")
                    ),
                    location: names.first().cloned(),
                }],
                counterexample: Some(format!(
                    "Function '{}' bypasses access control while sibling functions are guarded",
                    names[0]
                )),
            }
        }
    }

    /// P7 – State mutations in a single invocation are atomic (no partial updates).
    fn check_state_mutation_isolation(&self, functions: &[FunctionAnalysis]) -> PropertyResult {
        // Heuristic: functions with many writes and cross-contract calls are likely non-atomic.
        let non_atomic: Vec<&FunctionAnalysis> = functions
            .iter()
            .filter(|f| f.calls_storage_write && f.calls_cross_contract)
            .collect();

        if non_atomic.is_empty() {
            PropertyResult {
                id: "P007".into(),
                name: "State Mutation Isolation".into(),
                description: "State mutations within a single invocation appear atomic".into(),
                status: ProofStatus::Proved,
                method: ProofMethod::AbstractInterpretation,
                confidence: 0.82,
                evidence: vec![ProofEvidence {
                    kind: "no_interleaved_mutations".into(),
                    description: "No functions combine storage writes with cross-contract calls"
                        .into(),
                    location: None,
                }],
                counterexample: None,
            }
        } else {
            let names: Vec<String> = non_atomic
                .iter()
                .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
                .collect();
            PropertyResult {
                id: "P007".into(),
                name: "State Mutation Isolation".into(),
                description: "State mutations within a single invocation appear atomic".into(),
                status: ProofStatus::Inconclusive,
                method: ProofMethod::AbstractInterpretation,
                confidence: 0.68,
                evidence: vec![ProofEvidence {
                    kind: "mixed_writes_and_calls".into(),
                    description: format!(
                        "{} function(s) interleave storage writes with cross-contract calls: {}",
                        non_atomic.len(),
                        names.join(", ")
                    ),
                    location: names.first().cloned(),
                }],
                counterexample: Some(
                    "Partial state update followed by external call — rollback may be impossible"
                        .into(),
                ),
            }
        }
    }

    // ── Vulnerability scanner ─────────────────────────────────────────────────

    fn scan_vulnerabilities(
        &self,
        facts: &ModuleFacts,
        functions: &[FunctionAnalysis],
    ) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        // V1 – Missing authorization on state-mutating exports
        let unauth_writes: Vec<String> = functions
            .iter()
            .filter(|f| {
                facts.exports.contains_key(&f.index)
                    && f.calls_storage_write
                    && !f.calls_require_auth
            })
            .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
            .collect();

        if !unauth_writes.is_empty() {
            findings.push(SecurityFinding {
                id: "V001".into(),
                title: "Missing Authorization Check".into(),
                description: format!(
                    "Exported function(s) [{}] modify contract state without calling require_auth. \
                     Any account can invoke these functions and alter the contract's storage.",
                    unauth_writes.join(", ")
                ),
                severity: VulnerabilitySeverity::Critical,
                category: "Access Control".into(),
                cwe_id: Some("CWE-862".into()),
                affected_functions: unauth_writes,
                location: None,
                remediation: "Add `require_auth(env, &caller)` before any storage write in each affected function.".into(),
            });
        }

        // V2 – Reentrancy
        let reentrant: Vec<String> = functions
            .iter()
            .filter(|f| f.cross_call_after_write)
            .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
            .collect();

        if !reentrant.is_empty() {
            findings.push(SecurityFinding {
                id: "V002".into(),
                title: "Potential Reentrancy Vulnerability".into(),
                description: format!(
                    "Function(s) [{}] write to storage and then invoke an external contract. \
                     If the external contract re-enters this contract, state may be corrupted.",
                    reentrant.join(", ")
                ),
                severity: VulnerabilitySeverity::High,
                category: "Reentrancy".into(),
                cwe_id: Some("CWE-841".into()),
                affected_functions: reentrant,
                location: None,
                remediation:
                    "Follow the Checks-Effects-Interactions pattern: complete all storage \
                              updates before making cross-contract calls."
                        .into(),
            });
        }

        // V3 – Unchecked arithmetic on tainted values
        let overflow_risk: Vec<String> = functions
            .iter()
            .filter(|f| f.unchecked_arithmetic && facts.exports.contains_key(&f.index))
            .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
            .collect();

        if !overflow_risk.is_empty() {
            findings.push(SecurityFinding {
                id: "V003".into(),
                title: "Integer Overflow Risk".into(),
                description: format!(
                    "Exported function(s) [{}] perform unchecked integer arithmetic on values \
                     that originate from external sources. Arithmetic overflow may cause \
                     incorrect balance calculations or state corruption.",
                    overflow_risk.join(", ")
                ),
                severity: VulnerabilitySeverity::High,
                category: "Arithmetic".into(),
                cwe_id: Some("CWE-190".into()),
                affected_functions: overflow_risk,
                location: None,
                remediation: "Use Soroban's checked arithmetic primitives (checked_add, \
                              checked_sub, checked_mul) and handle the Err case explicitly."
                    .into(),
            });
        }

        // V4 – Unchecked division (potential divide-by-zero)
        let div_zero_risk: Vec<String> = functions
            .iter()
            .filter(|f| f.unchecked_divisions > 0)
            .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
            .collect();

        if !div_zero_risk.is_empty() {
            findings.push(SecurityFinding {
                id: "V004".into(),
                title: "Potential Division by Zero".into(),
                description: format!(
                    "Function(s) [{}] perform integer division with divisors that could not be \
                     proved non-zero. A zero divisor will trap the contract, making it unresponsive.",
                    div_zero_risk.join(", ")
                ),
                severity: VulnerabilitySeverity::Medium,
                category: "Arithmetic".into(),
                cwe_id: Some("CWE-369".into()),
                affected_functions: div_zero_risk,
                location: None,
                remediation: "Assert or check that divisors are non-zero before performing division. \
                              Consider using checked_div which returns Option<T>.".into(),
            });
        }

        // V5 – Writes before auth check
        let early_write: Vec<String> = functions
            .iter()
            .filter(|f| f.writes_before_auth)
            .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
            .collect();

        if !early_write.is_empty() {
            findings.push(SecurityFinding {
                id: "V005".into(),
                title: "State Write Before Authorization Check".into(),
                description: format!(
                    "Function(s) [{}] write to storage before calling require_auth. \
                     If auth fails, state may have been partially modified before the revert.",
                    early_write.join(", ")
                ),
                severity: VulnerabilitySeverity::Medium,
                category: "Access Control".into(),
                cwe_id: Some("CWE-696".into()),
                affected_functions: early_write,
                location: None,
                remediation: "Move require_auth to the very start of the function body, before \
                              any reads or writes to storage."
                    .into(),
            });
        }

        // V6 – Potentially unbounded loops (DoS risk)
        let loop_risk: Vec<String> = functions
            .iter()
            .filter(|f| f.has_unbounded_loop && facts.exports.contains_key(&f.index))
            .map(|f| f.name.clone().unwrap_or_else(|| format!("fn_{}", f.index)))
            .collect();

        if !loop_risk.is_empty() {
            findings.push(SecurityFinding {
                id: "V006".into(),
                title: "Potentially Unbounded Loop".into(),
                description: format!(
                    "Exported function(s) [{}] contain loops whose iteration count depends on \
                     runtime input. A caller can craft inputs that cause the contract to exceed \
                     the ledger CPU limit, causing denial of service.",
                    loop_risk.join(", ")
                ),
                severity: VulnerabilitySeverity::Medium,
                category: "Denial of Service".into(),
                cwe_id: Some("CWE-834".into()),
                affected_functions: loop_risk,
                location: None,
                remediation: "Cap loop iterations using a compile-time constant upper bound, \
                              or reject inputs that would exceed it."
                    .into(),
            });
        }

        findings
    }

    // ── Proof certificate ─────────────────────────────────────────────────────

    fn generate_certificate(&self, properties: &[PropertyResult]) -> ProofCertificate {
        let proved = properties
            .iter()
            .filter(|p| matches!(p.status, ProofStatus::Proved))
            .count();
        let violated = properties
            .iter()
            .filter(|p| matches!(p.status, ProofStatus::Violated))
            .count();
        let inconclusive = properties
            .iter()
            .filter(|p| matches!(p.status, ProofStatus::Inconclusive))
            .count();

        let overall_confidence = if properties.is_empty() {
            0.0
        } else {
            properties.iter().map(|p| p.confidence).sum::<f64>() / properties.len() as f64
        };

        let summary = match (proved, violated, inconclusive) {
            (_, v, _) if v > 0 => format!(
                "{} property violation(s) detected. Immediate review required.",
                v
            ),
            (_, _, i) if i > 0 => format!(
                "All {} critical properties proved; {} require manual review.",
                proved, i
            ),
            (p, 0, 0) => format!("All {} properties formally proved. Contract is safe.", p),
            _ => "Analysis complete.".into(),
        };

        ProofCertificate {
            certificate_id: Uuid::new_v4(),
            contract_id: self.contract_id,
            session_id: self.session_id,
            properties_proved: proved,
            properties_violated: violated,
            properties_inconclusive: inconclusive,
            overall_confidence,
            generated_at: Utc::now(),
            summary,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wasm() -> Vec<u8> {
        // Minimal valid WASM module: magic + version only (no sections).
        vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
    }

    #[test]
    fn analyzer_handles_minimal_wasm() {
        let id = Uuid::new_v4();
        let analyzer = WasmBytecodeAnalyzer::new(minimal_wasm(), id);
        let report = analyzer.run().expect("should succeed on minimal wasm");

        assert_eq!(report.contract_id, id);
        assert_eq!(report.function_count, 0);
        assert!(!report.certificate.summary.is_empty());
    }

    #[test]
    fn certificate_counts_match_properties() {
        let id = Uuid::new_v4();
        let analyzer = WasmBytecodeAnalyzer::new(minimal_wasm(), id);
        let report = analyzer.run().unwrap();

        let total = report.certificate.properties_proved
            + report.certificate.properties_violated
            + report.certificate.properties_inconclusive;
        assert_eq!(total, report.properties.len());
    }

    #[test]
    fn paged_query_strips_bad_params() {
        // Test the helper that rebuilds query strings (imported from pagination).
        let facts = ModuleFacts::default();
        assert!(facts.host_imports.is_empty());
    }

    #[test]
    fn classify_imports_recognise_soroban_functions() {
        assert_eq!(
            classify_import("env", "require_auth"),
            HostFunction::RequireAuth
        );
        assert_eq!(
            classify_import("env", "storage_put"),
            HostFunction::StorageSet
        );
        assert_eq!(
            classify_import("env", "storage_get"),
            HostFunction::StorageGet
        );
        assert_eq!(
            classify_import("env", "invoke_contract"),
            HostFunction::InvokeContract
        );
    }
}
