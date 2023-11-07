use std::fmt::Debug;
use biodivine_lib_bdd::{Bdd, BddPartialValuation};
use biodivine_lib_logical_models::{BinaryIntegerDomain, count_states_exact, find_start_of, GrayCodeIntegerDomain, PetriNetIntegerDomain, pick_state, SmartSystemUpdateFn, SymbolicDomain, SymbolicTransitionFn, UnaryIntegerDomain};

/// This binary is testing the implementation correctness by running reachability on the
/// input model and validating that the set of reachable states has the same cardinality
/// in every step.
///
/// For larger models, this is almost sure to take "forever", hence if you want to use it as
/// an automated test, you should always run it with a timeout, and ideally with optimizations.
/// This is also the reason why we don't use it as a normal integration test: because those
/// run unoptimized by default, and timeout can be only used to fail tests.
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let sbml_path = args[1].clone();

    let mut cmp = ComputationStep::new(sbml_path.as_str());
    loop {
        cmp.perform_step();
        cmp.check_consistency();
        if cmp.result_unary.is_none() {
            // The results must be equal because consistency check passed.
            println!("Test completed successfully.");
            return;
        }
    }
}

struct ComputationStep {
    steps: usize,
    result_unary: Option<Bdd>,
    result_binary: Option<Bdd>,
    result_gray: Option<Bdd>,
    result_petri_net: Option<Bdd>,
    system_unary: SmartSystemUpdateFn<UnaryIntegerDomain, u8>,
    system_binary: SmartSystemUpdateFn<BinaryIntegerDomain<u8>, u8>,
    system_gray: SmartSystemUpdateFn<GrayCodeIntegerDomain<u8>, u8>,
    system_petri_net: SmartSystemUpdateFn<PetriNetIntegerDomain, u8>,
}

/// Perform one step of backward reachability procedure. Returns either a new [Bdd] value, or
/// `None` if no new predecessors can be included.
fn bwd_step<D: SymbolicDomain<u8> + Debug>(system: &SmartSystemUpdateFn<D, u8>, set: &Bdd) -> Option<Bdd> {
    let sorted_variables = system.get_system_variables();

    for var in sorted_variables.iter().rev() {
        let predecessors = system.transition_under_variable(var.as_str(), set);

        // Should be equivalent to "predecessors \not\subseteq result".
        if !predecessors.imp(set).is_true() {
            let result = predecessors.or(&set);
            return Some(result);
        }
    }

    None
}

fn build_update_fn<D: SymbolicDomain<u8> + Debug>(sbml_path: &str) -> SmartSystemUpdateFn<D, u8> {
    let file = std::fs::File::open(sbml_path.clone())
        .expect("Cannot open SBML file.");
    let reader = std::io::BufReader::new(file);
    let mut xml = xml::reader::EventReader::new(reader);

    find_start_of(&mut xml, "listOfTransitions")
        .expect("Cannot find transitions in the SBML file.");

    let smart_system_update_fn = SmartSystemUpdateFn::<D, u8>::try_from_xml(&mut xml)
        .expect("Loading system fn update failed.");

    smart_system_update_fn
}

impl ComputationStep {

    pub fn new(sbml_path: &str) -> ComputationStep {
        let system_unary = build_update_fn::<UnaryIntegerDomain>(sbml_path);
        let system_binary = build_update_fn::<BinaryIntegerDomain<u8>>(sbml_path);
        let system_gray = build_update_fn::<GrayCodeIntegerDomain<u8>>(sbml_path);
        let system_petri_net = build_update_fn::<PetriNetIntegerDomain>(sbml_path);

        // As the initial state, we just set each variable to false, otherwise we'd go crazy
        // transferring the values between different encodings. (Also, we could just pick the
        // first available BDD valuation, but those are not necessarily equivalent across
        // encodings either).

        let mut result_unary = BddPartialValuation::empty();
        for var in system_unary.get_system_variables() {
            let domain = system_unary.named_symbolic_domains.get(&var).unwrap();
            domain.encode_bits(&mut result_unary, &0);
        }
        let result_unary = system_unary.get_bdd_variable_set().mk_conjunctive_clause(&result_unary);

        let mut result_binary = BddPartialValuation::empty();
        for var in system_binary.get_system_variables() {
            let domain = system_binary.named_symbolic_domains.get(&var).unwrap();
            domain.encode_bits(&mut result_binary, &0);
        }
        let result_binary = system_binary.get_bdd_variable_set().mk_conjunctive_clause(&result_binary);

        let mut result_gray = BddPartialValuation::empty();
        for var in system_gray.get_system_variables() {
            let domain = system_gray.named_symbolic_domains.get(&var).unwrap();
            domain.encode_bits(&mut result_gray, &0);
        }
        let result_gray = system_gray.get_bdd_variable_set().mk_conjunctive_clause(&result_gray);

        let mut result_petri_net = BddPartialValuation::empty();
        for var in system_petri_net.get_system_variables() {
            let domain = system_petri_net.named_symbolic_domains.get(&var).unwrap();
            domain.encode_bits(&mut result_petri_net, &0);
        }
        let result_petri_net = system_petri_net.get_bdd_variable_set().mk_conjunctive_clause(&result_petri_net);

        ComputationStep {
            steps: 0,
            result_unary: Some(result_unary),
            result_binary: Some(result_binary),
            result_gray: Some(result_gray),
            result_petri_net: Some(result_petri_net),
            system_unary,
            system_binary,
            system_gray,
            system_petri_net,
        }
    }

    pub fn perform_step(&mut self) {
        self.steps += 1;
        self.result_unary = bwd_step(&self.system_unary, self.result_unary.as_ref().unwrap());
        self.result_binary = bwd_step(&self.system_binary, self.result_binary.as_ref().unwrap());
        self.result_gray = bwd_step(&self.system_gray, self.result_gray.as_ref().unwrap());
        self.result_petri_net = bwd_step(&self.system_petri_net, self.result_petri_net.as_ref().unwrap());
    }

    pub fn check_consistency(&self) {
        let count_unary = self.result_unary.as_ref().map(|it| {
            count_states_exact(&self.system_unary, &it)
        });
        let count_binary = self.result_binary.as_ref().map(|it| {
            count_states_exact(&self.system_binary, &it)
        });
        let count_gray = self.result_gray.as_ref().map(|it| {
            count_states_exact(&self.system_gray, &it)
        });
        let count_petri_net = self.result_petri_net.as_ref().map(|it| {
            count_states_exact(&self.system_petri_net, &it)
        });
        if count_unary != count_binary || count_binary != count_gray || count_gray != count_petri_net {
            panic!(
                "Error at step {}. {:?} <> {:?} <> {:?} <> {:?}",
                self.steps,
                count_unary,
                count_binary,
                count_gray,
                count_petri_net
            )
        } else {
            println!("Step {} successful. Current result state count: {:?}", self.steps, count_unary);
            println!(
                " > BDD sizes: {:?} {:?} {:?} {:?}",
                self.result_unary.as_ref().map(|it| it.size()),
                self.result_binary.as_ref().map(|it| it.size()),
                self.result_gray.as_ref().map(|it| it.size()),
                self.result_petri_net.as_ref().map(|it| it.size()),
            );
        }
    }

}