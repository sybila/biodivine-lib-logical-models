#![allow(dead_code)]

pub struct SystemUpdateFn {}

pub mod variable_update_fn {
    use std::collections::HashMap;

    use biodivine_lib_bdd::{Bdd, BddVariable, BddVariableSet};

    use crate::{
        expression_components::{
            expression::Expression,
            proposition::{ComparisonOperator as CmpOp, Proposition},
        },
        symbolic_domains::symbolic_domain::SymbolicDomain,
        system::variable_update_function::VariableUpdateFn as UnprocessedFn,
    };

    pub struct VariableUpdateFn {
        pub bit_answering_bdds: Vec<(BddVariable, Bdd)>, // todo maybe add String aka the name associated with the BddVariable
    }

    impl VariableUpdateFn {
        pub fn from_update_fn<D, T>(
            update_fn: UnprocessedFn<T>,
            bdd_variable_set: &BddVariableSet,
            named_symbolic_domains: &HashMap<String, D>,
        ) -> Self
        where
            D: SymbolicDomain<T>,
        {
            let UnprocessedFn { terms, .. } = update_fn;

            let _todo_bdd_terms = terms.into_iter().map(|(val, match_condition)| {
                let match_condition_bdd =
                    bdd_from_expression(&match_condition, named_symbolic_domains, bdd_variable_set);
                (val, match_condition_bdd)
            });

            todo!()
        }
    }

    fn bdd_from_expression<D, T>(
        expression: &Expression<T>,
        named_symbolic_domains: &HashMap<String, D>,
        bdd_variable_set: &BddVariableSet,
    ) -> Bdd
    where
        D: SymbolicDomain<T>,
    {
        match expression {
            Expression::Terminal(proposition) => {
                bdd_from_proposition(proposition, named_symbolic_domains, bdd_variable_set)
            }
            _ => todo!(),
        }
    }

    fn bdd_from_proposition<D, T>(
        proposition: &Proposition<T>,
        named_symbolic_domains: &HashMap<String, D>,
        bdd_variable_set: &BddVariableSet,
    ) -> Bdd
    where
        D: SymbolicDomain<T>,
    {
        let target_vars_domain = named_symbolic_domains.get(&proposition.variable).unwrap_or_else(
            || panic!(
                "Symbolic domain for variable {} should be avilable, but is not; domains available only for variables [{}]",
                proposition.variable,
                named_symbolic_domains.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        );

        // todo maybe rename CmpOp to the full name (see its import here)
        match proposition.comparison_operator {
            CmpOp::Eq => target_vars_domain.encode_one_todo(bdd_variable_set, &proposition.value),
            CmpOp::Neq => target_vars_domain
                .encode_one_todo(bdd_variable_set, &proposition.value)
                .not(),
            CmpOp::Leq => lt(&proposition.value, target_vars_domain, bdd_variable_set),
            _ => todo!(),
        }
    }

    fn lt<D, T>(
        _lower_than_this: &T,
        _symbolic_domain: &D,
        _bdd_variable_set: &BddVariableSet,
    ) -> Bdd
    where
        D: SymbolicDomain<T>,
    {
        // (0..lower_than_this).fold(
        //     // todo this range will be problematic; either add restrictions for this impl block, or add another methods on D
        //     symbolic_domain.empty_collection_todo(bdd_variable_set),
        //     |acc, val| acc.or(&symbolic_domain.encode_one_todo(bdd_variable_set, &val)),
        // )

        todo!()
    }
}
