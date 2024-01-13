use biodivine_lib_bdd::{Bdd, BddPartialValuation};
use num_bigint::BigInt;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Shr;

// use crate::prototype::symbolic_domain::SymbolicDomain;

// use super::{SmartSystemUpdateFn, UpdateFn};

// use crate::{SmartSystemUpdateFn, SymbolicDomain, UpdateFn};

use crate::symbolic_domains::symbolic_domain::SymbolicDomain;
use crate::update::update_fn::SmartSystemUpdateFn;

/// Compute a [Bdd] which represents a single (un-primed) state within the given symbolic `set`.
pub fn pick_state_bdd<D: SymbolicDomain<u8> + Debug>(
    system: &SmartSystemUpdateFn<D, u8>,
    set: &Bdd,
) -> Bdd {
    // Unfortunately, this is now a bit more complicated than it needs to be, because
    // we have to ignore the primed variables, but it shouldn't bottleneck anything outside of
    // truly extreme cases.
    let standard_variables = system.standard_variables();
    let valuation = set
        .sat_witness()
        .expect("Cannot pick state from an empty set.");
    let mut state_data = BddPartialValuation::empty();
    for var in standard_variables {
        state_data.set_value(var, valuation.value(var))
    }
    system
        .get_bdd_variable_set()
        .mk_conjunctive_clause(&state_data)
}

/// Pick a state from a symbolic set and "decode" it into normal integers.
pub fn pick_state_map<D: SymbolicDomain<u8> + Debug>(
    system: &SmartSystemUpdateFn<D, u8>,
    set: &Bdd,
) -> HashMap<String, u8> {
    let valuation = set.sat_witness().expect("The set is empty.");
    let valuation = BddPartialValuation::from(valuation);
    let mut result = HashMap::new();
    for var in system.get_system_variables() {
        let domain = system.get_domain(&var).expect("known variables");
        let value = domain.decode_bits(&valuation);
        result.insert(var, value);
    }
    result
}

/// Encode a "state" (assignment of integer values to all variables) into a [Bdd] that is valid
/// within the provided [SmartSystemUpdateFn].
pub fn encode_state_map<D: SymbolicDomain<u8> + Debug>(
    system: &SmartSystemUpdateFn<D, u8>,
    state: &HashMap<String, u8>,
) -> Bdd {
    let mut result = BddPartialValuation::empty();
    for var in system.get_system_variables() {
        let Some(value) = state.get(&var) else {
            panic!("Value for {var} missing.");
        };
        let domain = system.get_domain(&var).expect("known variables");
        domain.encode_bits(&mut result, value);
    }
    system.get_bdd_variable_set().mk_conjunctive_clause(&result)
}

pub fn log_percent(set: &Bdd, universe: &Bdd) -> f64 {
    set.cardinality().log2() / universe.cardinality().log2() * 100.0
}

/// Compute an (approximate) count of state in the given `set` using the encoding of `system`.
pub fn count_states<D: SymbolicDomain<u8> + Debug>(
    system: &SmartSystemUpdateFn<D, u8>,
    set: &Bdd,
) -> f64 {
    let symbolic_var_count = system.get_bdd_variable_set().num_vars() as i32;
    // TODO:
    //   Here we assume that exactly half of the variables are primed, which may not be true
    //   in the future, but should be good enough for now.
    assert_eq!(symbolic_var_count % 2, 0);
    let primed_vars = symbolic_var_count / 2;
    set.cardinality() / 2.0f64.powi(primed_vars)
}

/// Same as [count_states], but with exact unbounded integers.
pub fn count_states_exact<D: SymbolicDomain<u8> + Debug>(
    system: &SmartSystemUpdateFn<D, u8>,
    set: &Bdd,
) -> BigInt {
    let symbolic_var_count = system.get_bdd_variable_set().num_vars() as i32;
    // TODO:
    //   Here we assume that exactly half of the variables are primed, which may not be true
    //   in the future, but should be good enough for now.
    assert_eq!(symbolic_var_count % 2, 0);
    let primed_vars = symbolic_var_count / 2;
    set.exact_cardinality().shr(primed_vars)
}
