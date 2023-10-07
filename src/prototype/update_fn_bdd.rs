use std::collections::HashMap;

use biodivine_lib_bdd::{
    Bdd, BddPartialValuation, BddValuation, BddVariableSet, BddVariableSetBuilder,
};

use crate::{Expression, SymbolicDomain, UpdateFn};

use super::expression::Proposition;

// todo this should be extracted from the xml/UpdateFn outputs for each of the variable in the system
const HARD_CODED_MAX_VAR_VALUE: u8 = 10;

// todo this should be abstacted into a geeneric parameter
// todo but that would require
// type ValueType = u8;

#[derive(Clone)]
pub struct UpdateFnBdd<D: SymbolicDomain<u8>> {
    // pub struct UpdateFnBdd_<D: SymbolicDomain<ValueType>> {
    pub target_var_name: String,
    pub terms: Vec<(u8, Bdd)>,
    pub named_symbolic_domains: std::collections::HashMap<String, D>,
    pub default: u8,
    pub bdd_variable_set: BddVariableSet,
    pub result_domain: D,
}

// impl<D: SymbolicDomain<T>, T> From<UpdateFn> for UpdateFnBdd_<D, T> {
impl<D: SymbolicDomain<u8>> From<UpdateFn<u8>> for UpdateFnBdd<D> {
    fn from(update_fn: UpdateFn<u8>) -> Self {
        let mut bdd_variable_set_builder = BddVariableSetBuilder::new();

        let named_symbolic_domains = update_fn
            .input_vars_names
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    D::new(
                        &mut bdd_variable_set_builder,
                        name,
                        HARD_CODED_MAX_VAR_VALUE,
                    ),
                )
            })
            .collect();

        let bdd_variable_set = bdd_variable_set_builder.build();
        let terms = update_fn
            .terms
            .iter()
            .map(|(val, expr)| {
                (
                    val.to_owned(),
                    bdd_from_expr(expr.to_owned(), &named_symbolic_domains, &bdd_variable_set),
                )
            })
            .collect();

        let max_output = update_fn
            .terms
            .iter()
            .map(|(val, _)| val)
            .chain(std::iter::once(&update_fn.default))
            // todo not really interested in `max` per se, but in the one requiring larges number of bits
            .max()
            .unwrap(); // there will always be at least &source.default

        // todo this will likely be shared between all the update fns
        // todo but for now, this variable must not be in together with the rest of the symbolic domains
        let result_bdd_variable_set_domain = &mut BddVariableSetBuilder::new();
        let result_domain = D::new(
            result_bdd_variable_set_domain,
            &update_fn.target_var_name,
            max_output.to_owned(),
        );

        Self {
            target_var_name: update_fn.target_var_name,
            terms,
            named_symbolic_domains,
            default: update_fn.default,
            bdd_variable_set,
            result_domain,
        }
    }
}

impl<D: SymbolicDomain<u8>> UpdateFnBdd<D> {
    pub fn from_update_fn(
        update_fn: UpdateFn<u8>,
        bdd_variable_set: &BddVariableSet,
        named_symbolic_domains: &HashMap<String, D>,
    ) -> Self {
        let terms = update_fn
            .terms
            .iter()
            .map(|(val, expr)| {
                (
                    val.to_owned(),
                    bdd_from_expr(expr.to_owned(), named_symbolic_domains, bdd_variable_set),
                )
            })
            .collect::<Vec<_>>();

        Self {
            target_var_name: update_fn.target_var_name.clone(),
            terms,
            named_symbolic_domains: named_symbolic_domains.clone(),
            default: update_fn.default,
            bdd_variable_set: bdd_variable_set.clone(),
            result_domain: named_symbolic_domains
                .get(&update_fn.target_var_name)
                .unwrap()
                .clone(),
        }
    }
}

fn bdd_from_expr<D: SymbolicDomain<u8>>(
    // fn bdd_from_expr<D: SymbolicDomain<ValueType>>(
    expr: &Expression<u8>,
    symbolic_domains: &HashMap<String, D>,
    bdd_variable_set: &BddVariableSet,
) -> Bdd {
    match expr {
        // prop_to_bdd is the important thing here;
        // the rest is just recursion & calling the right bdd methods
        Expression::Terminal(prop) => {
            prop_to_bdd(prop.to_owned(), symbolic_domains, bdd_variable_set)
        }
        Expression::Not(expr) => {
            let bdd = bdd_from_expr(expr, symbolic_domains, bdd_variable_set);
            bdd.not()
        }
        Expression::And(clauses) => clauses.iter().fold(bdd_variable_set.mk_true(), |acc, it| {
            acc.and(&bdd_from_expr(it, symbolic_domains, bdd_variable_set))
        }),
        Expression::Or(clauses) => clauses.iter().fold(bdd_variable_set.mk_false(), |acc, it| {
            acc.or(&bdd_from_expr(it, symbolic_domains, bdd_variable_set))
        }),
        Expression::Xor(lhs, rhs) => {
            let lhs = bdd_from_expr(lhs, symbolic_domains, bdd_variable_set);
            let rhs = bdd_from_expr(rhs, symbolic_domains, bdd_variable_set);
            lhs.xor(&rhs)
        }
        Expression::Implies(lhs, rhs) => {
            let lhs = bdd_from_expr(lhs, symbolic_domains, bdd_variable_set);
            let rhs = bdd_from_expr(rhs, symbolic_domains, bdd_variable_set);
            lhs.imp(&rhs)
        }
    }
}

// todo this should be applied to each term directly while loading the xml; no need to even have the intermediate representation
// todo actually it might not be bad idea to keep the intermediate repr for now; debugging
fn prop_to_bdd<D: SymbolicDomain<u8>>(
    // fn prop_to_bdd<D: SymbolicDomain<ValueType>>(
    prop: Proposition<u8>,
    symbolic_domains: &HashMap<String, D>,
    bdd_variable_set: &BddVariableSet,
) -> Bdd {
    // let var = symbolic_domains.get(&prop.ci).unwrap();

    let var = match symbolic_domains.get(&prop.ci) {
        None => {
            panic!(
                "looking for {:?} but only {:?} present",
                prop.ci,
                symbolic_domains.keys().collect::<Vec<_>>()
            );
        }
        Some(var) => var,
    };

    let val = prop.cn;

    match prop.cmp {
        super::expression::CmpOp::Eq => var.encode_one(bdd_variable_set, &val),
        super::expression::CmpOp::Neq => var.encode_one(bdd_variable_set, &val).not(),
        super::expression::CmpOp::Lt => lt(var, bdd_variable_set, val),
        super::expression::CmpOp::Leq => leq(var, bdd_variable_set, val),
        super::expression::CmpOp::Gt => leq(var, bdd_variable_set, val).not(),
        super::expression::CmpOp::Geq => lt(var, bdd_variable_set, val).not(),
    }
}

fn lt<D: SymbolicDomain<u8>>(
    // fn lt<D: SymbolicDomain<ValueType>>(
    symbolic_domain: &D,
    bdd_variable_set: &BddVariableSet,
    lower_than_this: u8,
) -> Bdd {
    let mut bdd = symbolic_domain.empty_collection(bdd_variable_set);

    (0..lower_than_this).for_each(|i| {
        let bdd_i = symbolic_domain.encode_one(bdd_variable_set, &i);
        bdd = bdd.or(&bdd_i);
    });

    bdd
}

fn leq<D: SymbolicDomain<u8>>(
    symbolic_domain: &D,
    bdd_variable_set: &BddVariableSet,
    lower_or_same_as_this: u8,
) -> Bdd {
    let mut bdd = symbolic_domain.empty_collection(bdd_variable_set);

    (0..=lower_or_same_as_this).for_each(|i| {
        let bdd_i = symbolic_domain.encode_one(bdd_variable_set, &i);
        bdd = bdd.or(&bdd_i);
    });

    bdd
}

impl<D: SymbolicDomain<u8>> UpdateFnBdd<D> {
    /// for given valuation of input variables, returns the value of the output variable according to the update function
    /// todo should probably accept valuations of the symbolic variables
    /// todo so that user is abstracted from having to specify vector of bools
    /// todo and instead can just specify the values of symbolic variables
    /// todo for now, i know what is the underlying representation of the symbolic variables
    /// todo -> good enough for testing
    pub fn eval_in(&self, valuation: &BddValuation) -> u8 {
        self.terms
            .iter()
            .find(|(_, bdd)| bdd.eval_in(valuation))
            .map(|(val, _)| *val)
            .unwrap_or(self.default)
    }
}

impl<D: SymbolicDomain<u8>> UpdateFnBdd<D> {
    /// returns fully specified valuation representing all the symbolic variables
    /// being set to 0
    /// but also this valuation is partial, so that it can be updated later
    /// since all are set, you can build BddValuation from it at any time and
    /// evaluate the update function using this
    pub fn get_default_valuation_but_partial(&self) -> BddPartialValuation {
        self.named_symbolic_domains.values().fold(
            BddPartialValuation::empty(),
            |mut acc, domain| {
                domain.encode_bits(&mut acc, &0);
                acc
            },
        )
    }
}
