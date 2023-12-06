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
        symbolic_domains::symbolic_domain::SymbolicDomainOrd,
        system::variable_update_function::VariableUpdateFn as UnprocessedFn,
    };

    pub struct VariableUpdateFn {
        pub bit_answering_bdds: Vec<(BddVariable, Bdd)>, // todo maybe add String aka the name associated with the BddVariable
    }

    impl VariableUpdateFn {
        /// target_variable_name is a key in named_symbolic_domains
        pub fn from_update_fn<DO, T>(
            update_fn: UnprocessedFn<T>,
            target_variable_name: &str,
            bdd_variable_set: &BddVariableSet,
            named_symbolic_domains: &HashMap<String, DO>,
        ) -> Self
        where
            DO: SymbolicDomainOrd<T>,
        {
            let UnprocessedFn { terms, default, .. } = update_fn;

            let (outputs, bdd_conds): (Vec<_>, Vec<_>) = terms
                .into_iter()
                .map(|(val, match_condition)| {
                    let match_condition_bdd = bdd_from_expression(
                        &match_condition,
                        named_symbolic_domains,
                        bdd_variable_set,
                    );
                    (val, match_condition_bdd)
                })
                .chain(Some((default, bdd_variable_set.mk_true())))
                .unzip();

            let (_, values_mutally_exclusive_terms) = bdd_conds.into_iter().fold(
                (bdd_variable_set.mk_false(), Vec::new()),
                |(seen_states, mut acc), term_bdd| {
                    let mutually_exclusive_bdd = term_bdd.and(&seen_states.not());

                    let updated_ctx_seen_states = seen_states.or(&term_bdd);
                    acc.push(mutually_exclusive_bdd);

                    (updated_ctx_seen_states, acc)
                },
            );

            let target_domain = named_symbolic_domains
                .get(target_variable_name)
                .expect("must know the domain of the target variable");

            let bit_matrix = outputs
                .into_iter()
                .map(|output| target_domain.encode_bits_inspect(&output))
                .collect::<Vec<_>>();

            let bit_answering_bdds = (0..bit_matrix[0].len()).map(|bit_idx| {
                (0..bit_matrix.len()).fold(bdd_variable_set.mk_false(), |acc, row_idx| {
                    if bit_matrix[row_idx][bit_idx] {
                        acc.or(&values_mutally_exclusive_terms[row_idx])
                    } else {
                        acc
                    }
                })
            });

            // todo consider ordering of the bits
            Self {
                bit_answering_bdds: target_domain
                    .symbolic_variables()
                    .into_iter()
                    .zip(bit_answering_bdds)
                    .collect(),
            }
        }
    }

    fn bdd_from_expression<DO, T>(
        expression: &Expression<T>,
        named_symbolic_domains: &HashMap<String, DO>,
        bdd_variable_set: &BddVariableSet,
    ) -> Bdd
    where
        DO: SymbolicDomainOrd<T>,
    {
        match expression {
            Expression::Terminal(proposition) => {
                bdd_from_proposition(proposition, named_symbolic_domains, bdd_variable_set)
            }
            Expression::Not(expression) => {
                bdd_from_expression(expression, named_symbolic_domains, bdd_variable_set).not()
            }
            Expression::And(clauses) => {
                clauses
                    .iter()
                    // todo one of the places where intersection with `unit_set` should be considered
                    .fold(bdd_variable_set.mk_true(), |acc, clausule| {
                        acc.and(&bdd_from_expression(
                            clausule,
                            named_symbolic_domains,
                            bdd_variable_set,
                        ))
                    })
            }
            Expression::Or(clauses) => {
                clauses
                    .iter()
                    .fold(bdd_variable_set.mk_false(), |acc, clausule| {
                        acc.or(&bdd_from_expression(
                            clausule,
                            named_symbolic_domains,
                            bdd_variable_set,
                        ))
                    })
            }
            Expression::Xor(lhs, rhs) => {
                let lhs = bdd_from_expression(lhs, named_symbolic_domains, bdd_variable_set);
                let rhs = bdd_from_expression(rhs, named_symbolic_domains, bdd_variable_set);
                lhs.xor(&rhs)
            }
            Expression::Implies(lhs, rhs) => {
                let lhs = bdd_from_expression(lhs, named_symbolic_domains, bdd_variable_set);
                let rhs = bdd_from_expression(rhs, named_symbolic_domains, bdd_variable_set);
                lhs.imp(&rhs)
            }
        }
    }

    fn bdd_from_proposition<DO, T>(
        proposition: &Proposition<T>,
        named_symbolic_domains: &HashMap<String, DO>,
        bdd_variable_set: &BddVariableSet,
    ) -> Bdd
    where
        DO: SymbolicDomainOrd<T>,
    {
        let target_vars_domain = named_symbolic_domains.get(&proposition.variable).unwrap_or_else(
            || panic!(
                "Symbolic domain for variable {} should be avilable, but is not; domains available only for variables [{}]",
                proposition.variable,
                named_symbolic_domains.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        );

        match proposition.comparison_operator {
            CmpOp::Eq => target_vars_domain.encode_one(bdd_variable_set, &proposition.value),
            CmpOp::Neq => target_vars_domain
                .encode_one(bdd_variable_set, &proposition.value)
                // todo one of the places where intersection with `unit_set` should be considered (or `domain.encode_one_not()`)
                .not(),
            CmpOp::Lt => target_vars_domain.encode_lt(bdd_variable_set, &proposition.value),
            CmpOp::Leq => target_vars_domain.encode_le(bdd_variable_set, &proposition.value),
            CmpOp::Gt => target_vars_domain.encode_gt(bdd_variable_set, &proposition.value),
            CmpOp::Geq => target_vars_domain.encode_ge(bdd_variable_set, &proposition.value),
        }
    }
}
