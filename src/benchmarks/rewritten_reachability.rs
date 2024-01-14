use biodivine_lib_bdd::Bdd;
use std::fmt::Debug;

use crate::{
    prelude::find_start_of, symbolic_domains::symbolic_domain::SymbolicDomainOrd,
    update::update_fn::SmartSystemUpdateFn as RewrittenSmartSystemUpdateFn,
};

pub fn log_percent(set: &Bdd, universe: &Bdd) -> f64 {
    set.cardinality().log2() / universe.cardinality().log2() * 100.0
}

pub fn reachability_benchmark<DO: SymbolicDomainOrd<u8> + Debug>(sbml_path: &str) {
    let smart_system_update_fn = {
        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));

        find_start_of(&mut xml, "listOfTransitions")
            .expect("Cannot find transitions in the SBML file.");

        RewrittenSmartSystemUpdateFn::<DO, u8>::try_from_xml(&mut xml)
            .expect("Loading system fn update failed.")
    };

    let unit = smart_system_update_fn.unit_vertex_set();
    let system_var_count = smart_system_update_fn.standard_variables().len();
    println!(
        "Variables: {}, expected states {}",
        system_var_count,
        1 << system_var_count
    );
    println!(
        "Computed state count: {}",
        smart_system_update_fn.count_states(&unit)
    );
    let mut universe = unit.clone();
    while !universe.is_false() {
        let mut weak_scc = smart_system_update_fn.pick_state_bdd(&universe);
        loop {
            let bwd_reachable = reach_bwd(&smart_system_update_fn, &weak_scc, &universe);
            let fwd_bwd_reachable = reach_fwd(&smart_system_update_fn, &bwd_reachable, &universe);

            // FWD/BWD reachable set is not a subset of weak SCC, meaning the SCC can be expanded.
            if !fwd_bwd_reachable.imp(&weak_scc).is_true() {
                println!(
                    " + SCC increased to (states={}, size={})",
                    smart_system_update_fn.count_states(&weak_scc),
                    weak_scc.size()
                );
                weak_scc = fwd_bwd_reachable;
            } else {
                break;
            }
        }
        println!(
            " + Found weak SCC (states={}, size={})",
            smart_system_update_fn.count_states(&weak_scc),
            weak_scc.size()
        );
        // Remove the SCC from the universe set and start over.
        universe = universe.and_not(&weak_scc);
        println!(
            " + Remaining states: {}/{}",
            smart_system_update_fn.count_states(&universe),
            smart_system_update_fn.count_states(&unit),
        );
    }
}

/// Compute the set of vertices that are forward-reachable from the `initial` set.
///
/// The result BDD contains a vertex `x` if and only if there is a (possibly zero-length) path
/// from some vertex `x' \in initial` into `x`, i.e. `x' -> x`.
pub fn reach_fwd<D: SymbolicDomainOrd<u8> + Debug>(
    system: &RewrittenSmartSystemUpdateFn<D, u8>,
    initial: &Bdd,
    universe: &Bdd,
) -> Bdd {
    // The list of system variables, sorted in descending order (i.e. opposite order compared
    // to the ordering inside BDDs).
    let sorted_variables = system.get_system_variables();
    let mut result = initial.clone();
    println!(
        "Start forward reachability: (states={}, size={})",
        system.count_states(&result),
        result.size()
    );
    'fwd: loop {
        for var in sorted_variables.iter().rev() {
            let successors = system.successors_async(var.as_str(), &result);

            // Should be equivalent to "successors \not\subseteq result".
            if !successors.imp(&result).is_true() {
                result = result.or(&successors);
                println!(
                    " >> (progress={:.2}%%, states={}, size={})",
                    log_percent(&result, universe),
                    system.count_states(&result),
                    result.size()
                );
                continue 'fwd;
            }
        }

        // No further successors were computed across all variables. We are done.
        println!(
            " >> Done. (states={}, size={})",
            system.count_states(&result),
            result.size()
        );
        return result;
    }
}

/// Compute the set of vertices that are backward-reachable from the `initial` set.
///
/// The result BDD contains a vertex `x` if and only if there is a (possibly zero-length) path
/// from `x` into some vertex `x' \in initial`, i.e. `x -> x'`.
pub fn reach_bwd<D: SymbolicDomainOrd<u8> + Debug>(
    system: &RewrittenSmartSystemUpdateFn<D, u8>,
    initial: &Bdd,
    universe: &Bdd,
) -> Bdd {
    let sorted_variables = system.get_system_variables();
    let mut result = initial.clone();
    println!(
        "Start backward reachability: (states={}, size={})",
        system.count_states(&result),
        result.size()
    );
    'bwd: loop {
        for var in sorted_variables.iter().rev() {
            let predecessors = system.predecessors_async(var.as_str(), result.clone());

            // Should be equivalent to "predecessors \not\subseteq result".
            if !predecessors.imp(&result).is_true() {
                result = result.or(&predecessors);
                println!(
                    " >> (progress={:.2}%%, states={}, size={})",
                    log_percent(&result, universe),
                    system.count_states(&result),
                    result.size()
                );
                continue 'bwd;
            }
        }

        // No further predecessors were computed across all variables. We are done.
        println!(
            " >> Done. (states={}, size={})",
            system.count_states(&result),
            result.size()
        );
        return result;
    }
}
