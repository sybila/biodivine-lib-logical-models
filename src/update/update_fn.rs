#![allow(dead_code)]

use std::collections::HashMap;

use biodivine_lib_bdd::{BddVariableSet, BddVariableSetBuilder};

use crate::{
    expression_components::{expression::Expression, proposition::Proposition},
    symbolic_domains::symbolic_domain::{SymbolicDomain, SymbolicDomainOrd},
    system::variable_update_function::UnprocessedVariableUpdateFn,
};

use self::variable_update_fn::VariableUpdateFn;

pub struct SystemUpdateFn<D, T>
where
    D: SymbolicDomain<T>,
{
    /// ordered by variable name // todo add a method to get the update function by name (hash map or binary search)
    update_fns: Vec<(String, (VariableUpdateFn, D))>,
    bdd_variable_set: BddVariableSet,
    marker: std::marker::PhantomData<T>,
}

impl<DO, T> SystemUpdateFn<DO, T>
where
    DO: SymbolicDomainOrd<T>,
{
    pub fn from_update_fns(
        // todo do not forget to add default update functions for those variables that are not updated (in the loader from xml)
        vars_and_their_update_fns: HashMap<String, UnprocessedVariableUpdateFn<T>>,
    ) -> Self {
        let named_update_fns_sorted = {
            let mut to_be_sorted = vars_and_their_update_fns.into_iter().collect::<Vec<_>>();
            to_be_sorted.sort_by_key(|(var_name, _)| var_name.clone());
            to_be_sorted
        };

        let (symbolic_domains, bdd_variable_set) = {
            let max_values = Self::find_max_values(&named_update_fns_sorted);
            let (symbolic_domains, variable_set_builder) = named_update_fns_sorted.iter().fold(
                (Vec::new(), BddVariableSetBuilder::new()),
                |(mut domains, mut variable_set), (var_name, _update_fn)| {
                    let max_value = max_values
                        .get(var_name.as_str())
                        .expect("max value always present");

                    let domain = DO::new(&mut variable_set, var_name, max_value);
                    domains.push(domain);
                    (domains, variable_set)
                },
            );

            (symbolic_domains, variable_set_builder.build())
        };

        let named_symbolic_domains = named_update_fns_sorted
            .iter()
            .zip(symbolic_domains.iter())
            .map(|((var_name, _), domain)| (var_name.as_str(), domain))
            .collect::<HashMap<_, _>>();

        let update_fns = named_update_fns_sorted
            .iter()
            .map(|(var_name, update_fn)| {
                VariableUpdateFn::from_update_fn(
                    update_fn,
                    var_name,
                    &bdd_variable_set,
                    &named_symbolic_domains,
                )
            })
            .collect::<Vec<_>>();

        let the_triple = named_update_fns_sorted
            .into_iter()
            .zip(update_fns)
            .zip(symbolic_domains)
            .map(|(((var_name, _), update_fn), domain)| (var_name, (update_fn, domain)))
            .collect::<Vec<_>>();

        Self {
            update_fns: the_triple,
            bdd_variable_set,
            marker: std::marker::PhantomData,
        }
    }

    fn find_max_values(
        vars_and_their_update_fns: &[(String, UnprocessedVariableUpdateFn<T>)],
    ) -> HashMap<&str, &T> {
        let xd = vars_and_their_update_fns.iter().fold(
            HashMap::new(),
            |mut acc, (var_name, update_fn)| {
                let max_value = update_fn
                    .terms
                    .iter()
                    .map(|(val, _)| val)
                    .chain(Some(&update_fn.default))
                    .max_by(|x, y| DO::cmp(x, y))
                    .expect("default value always present");
                // no balls
                // // SAFETY: there is always at least the default value
                // let max_value = unsafe { max_value_option.unwrap_unchecked() };
                acc.insert(var_name.as_str(), max_value);
                acc
            },
        );

        vars_and_their_update_fns
            .iter()
            .flat_map(|(_var_name, update_fn)| update_fn.terms.iter().map(|(_, expr)| expr))
            .fold(xd, |mut acc, expr| {
                update_max::<DO, T>(&mut acc, expr);
                acc
            })
    }
}

fn update_max<'a, DO, T>(acc: &mut HashMap<&'a str, &'a T>, expr: &'a Expression<T>)
where
    DO: SymbolicDomainOrd<T>,
{
    match expr {
        Expression::Terminal(proposition) => {
            update_from_proposition::<DO, T>(acc, proposition);
        }
        Expression::Not(expression) => {
            update_max::<DO, T>(acc, expression);
        }
        Expression::And(clauses) | Expression::Or(clauses) => {
            clauses
                .iter()
                .for_each(|clause| update_max::<DO, T>(acc, clause));
        }
        Expression::Xor(lhs, rhs) | Expression::Implies(lhs, rhs) => {
            update_max::<DO, T>(acc, lhs);
            update_max::<DO, T>(acc, rhs);
        }
    }
}

fn update_from_proposition<'a, DO, T>(
    acc: &mut HashMap<&'a str, &'a T>,
    proposition: &'a Proposition<T>,
) where
    DO: SymbolicDomainOrd<T>,
{
    let Proposition {
        variable, value, ..
    } = proposition;

    acc.entry(variable.as_str())
        .and_modify(|old_val| {
            if DO::cmp(old_val, value) == std::cmp::Ordering::Less {
                *old_val = value
            }
        })
        .or_insert(value);
}

pub mod variable_update_fn {
    use std::collections::HashMap;

    use biodivine_lib_bdd::{Bdd, BddVariable, BddVariableSet};

    use crate::{
        expression_components::{
            expression::Expression,
            proposition::{ComparisonOperator as CmpOp, Proposition},
        },
        symbolic_domains::symbolic_domain::SymbolicDomainOrd,
        system::variable_update_function::UnprocessedVariableUpdateFn as UnprocessedFn,
    };

    pub struct VariableUpdateFn {
        pub bit_answering_bdds: Vec<(BddVariable, Bdd)>, // todo maybe add String aka the name associated with the BddVariable
    }

    impl VariableUpdateFn {
        /// target_variable_name is a key in named_symbolic_domains
        pub fn from_update_fn<DO, T>(
            update_fn: &UnprocessedFn<T>,
            target_variable_name: &str,
            bdd_variable_set: &BddVariableSet,
            named_symbolic_domains: &HashMap<&str, &DO>,
        ) -> Self
        where
            DO: SymbolicDomainOrd<T>,
        {
            let UnprocessedFn { terms, default, .. } = update_fn;

            let (outputs, bdd_conds): (Vec<_>, Vec<_>) = terms
                .iter()
                .map(|(val, match_condition)| {
                    let match_condition_bdd = bdd_from_expression(
                        match_condition,
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
                .map(|output| target_domain.encode_bits_inspect(output))
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
        named_symbolic_domains: &HashMap<&str, &DO>,
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
        named_symbolic_domains: &HashMap<&str, &DO>,
        bdd_variable_set: &BddVariableSet,
    ) -> Bdd
    where
        DO: SymbolicDomainOrd<T>,
    {
        let target_vars_domain = named_symbolic_domains.get(proposition.variable.as_str()).unwrap_or_else(
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