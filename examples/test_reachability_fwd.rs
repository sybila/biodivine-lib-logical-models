use std::fmt::Debug;
use biodivine_lib_logical_models::SymbolicDomain;
use biodivine_lib_logical_models::test_utils::ComputationStep;


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
    while !cmp.is_done() {
        cmp.initialize();
        while !cmp.can_initialize() {
            cmp.perform_fwd_step();
            cmp.check_consistency();
        }
        println!("Completed one wave of reachability. Reinitializing...")
    }
    println!("Test completed successfully. Whole state space explored.");
}
