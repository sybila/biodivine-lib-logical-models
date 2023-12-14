#![allow(dead_code)]

use std::fmt::Debug;

use biodivine_lib_bdd::{Bdd, BddPartialValuation};

use crate::{
    symbolic_domains::symbolic_domain::SymbolicDomainOrd,
    update::update_fn::SmartSystemUpdateFn as RewrittenSmartSystemUpdateFn,
};

pub fn states<D: SymbolicDomainOrd<u8>>(
    system: &RewrittenSmartSystemUpdateFn<D, u8>,
    set: &Bdd,
) -> f64 {
    let symbolic_var_count = system.bdd_variable_set.num_vars() as i32;
    // TODO:
    //   Here we assume that exactly half of the variables are primed, which may not be true
    //   in the future, but should be good enough for now.
    assert_eq!(symbolic_var_count % 2, 0);
    let primed_vars = symbolic_var_count / 2;
    set.cardinality() / 2.0f64.powi(primed_vars)
}

pub fn unit_vertex_set<D: SymbolicDomainOrd<u8>>(
    system: &RewrittenSmartSystemUpdateFn<D, u8>,
) -> Bdd {
    system
        .variables_transition_relation_and_domain
        .iter()
        .fold(system.bdd_variable_set.mk_true(), |acc, (_, var_info)| {
            acc.and(&var_info.domain.unit_collection(&system.bdd_variable_set))
        })
}

/// Compute an (approximate) count of state in the given `set` using the encoding of `system`.
pub fn count_states<D: SymbolicDomainOrd<u8> + Debug>(
    system: &RewrittenSmartSystemUpdateFn<D, u8>,
    set: &Bdd,
) -> f64 {
    let symbolic_var_count = system.variables_transition_relation_and_domain.len() as i32;
    set.cardinality() / 2.0f64.powi(symbolic_var_count)
}

/// Compute a [Bdd] which represents a single (un-primed) state within the given symbolic `set`.
pub fn pick_state_bdd<D: SymbolicDomainOrd<u8> + Debug>(
    system: &RewrittenSmartSystemUpdateFn<D, u8>,
    set: &Bdd,
) -> Bdd {
    // Unfortunately, this is now a bit more complicated than it needs to be, because
    // we have to ignore the primed variables, but it shouldn't bottleneck anything outside of
    // truly extreme cases.
    let standard_variables = system
        .variables_transition_relation_and_domain
        .iter()
        .flat_map(|transition| transition.1.domain.raw_bdd_variables());
    let valuation = set
        .sat_witness()
        .expect("Cannot pick state from an empty set.");
    let mut state_data = BddPartialValuation::empty();
    for var in standard_variables {
        state_data.set_value(var, valuation.value(var))
    }
    system.bdd_variable_set.mk_conjunctive_clause(&state_data)
}

pub fn log_percent(set: &Bdd, universe: &Bdd) -> f64 {
    set.cardinality().log2() / universe.cardinality().log2() * 100.0
}
