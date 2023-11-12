use std::fmt::Debug;

use biodivine_lib_bdd::{Bdd, BddVariableSet};

use crate::prototype::symbolic_domain::SymbolicDomain;

use super::VariableUpdateFnCompiled;

#[derive(Debug)]
pub struct SymbolicTransitionFn<D: SymbolicDomain<T>, T> {
    pub transition_function: Bdd,
    penis: std::marker::PhantomData<T>,
    penis2: std::marker::PhantomData<D>,
}

impl<D: SymbolicDomain<T> + Debug, T> SymbolicTransitionFn<D, T> {
    pub fn from_update_fn_compiled(
        update_fn_compiled: &VariableUpdateFnCompiled<D, T>,
        ctx: &BddVariableSet,
        target_variable_name: &str,
    ) -> Self {
        let target_sym_dom = update_fn_compiled
            .named_symbolic_domains
            .get(target_variable_name)
            .expect("this symbolic variable/domain should be known");

        let target_sym_dom_primed = update_fn_compiled
            .named_symbolic_domains
            .get(&format!("{}'", target_variable_name))
            .expect("this symbolic variable/domain should be known");

        let mut accumulator = ctx.mk_true();

        for (bit_answering_bdd, bdd_variable) in &update_fn_compiled.bit_answering_bdds {
            let reconstructed_target_bdd_variable_name =
                crate::prototype::utils::find_bdd_variables_prime(
                    bdd_variable,
                    target_sym_dom,
                    target_sym_dom_primed,
                );

            let primed_target_variable_bdd = ctx.mk_var(reconstructed_target_bdd_variable_name);
            let the_part_of_the_update_fn = primed_target_variable_bdd.iff(bit_answering_bdd);

            // conjuct all the iff formulas
            accumulator = accumulator.and(&the_part_of_the_update_fn);
        }

        Self {
            transition_function: accumulator,
            penis: std::marker::PhantomData,
            penis2: std::marker::PhantomData,
        }
    }
}
