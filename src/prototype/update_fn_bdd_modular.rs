use std::collections::HashMap;

use biodivine_lib_bdd::{Bdd, BddVariableSet, BddVariableSetBuilder};

use crate::{Expression, SymbolicDomain, UpdateFn};

use super::expression::Proposition;

// todo this should be extracted from the xml/UpdateFn outputs for each of the variable in the system
const HARD_CODED_MAX_VAR_VALUE: u8 = 16;

// todo this should be abstacted into a geeneric parameter
// todo but that would require
type ValueType = u8;

#[derive(Clone)]
// pub struct UpdateFnBdd_<D: SymbolicDomain<ValueType>, ValueType> {
pub struct UpdateFnBdd_<D: SymbolicDomain<ValueType>> {
    pub target_var_name: String,
    pub terms: Vec<(ValueType, Bdd)>,
    pub named_symbolic_domains: std::collections::HashMap<String, D>,
    pub default: ValueType,
    pub bdd_variable_set: BddVariableSet,
    pub result_domain: D,
}

// impl<D: SymbolicDomain<T>, T> From<UpdateFn> for UpdateFnBdd_<D, T> {
impl<D: SymbolicDomain<ValueType>> From<UpdateFn> for UpdateFnBdd_<D> {
    fn from(update_fn: UpdateFn) -> Self {
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

        let mut bdd_variable_set = bdd_variable_set_builder.build();
        let terms = update_fn
            .terms
            .iter()
            .map(|(val, expr)| {
                (
                    val.to_owned(),
                    bdd_from_expr(
                        expr.to_owned(),
                        &named_symbolic_domains,
                        &mut bdd_variable_set,
                    ),
                )
            })
            .collect();

        let max_output = update_fn
            .terms
            .iter()
            .map(|(val, _)| val)
            .chain(std::iter::once(&update_fn.default))
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

// fn bdd_from_expr<D: SymbolicDomain<T>, T>(
fn bdd_from_expr<D: SymbolicDomain<ValueType>>(
    expr: &Expression,
    symbolic_domains: &HashMap<String, D>,
    bdd_variable_set: &mut BddVariableSet,
) -> Bdd {
    match expr {
        // prop_to_bdd is the important thing here;
        // the rest is just recursion & calling the right bdd methods
        Expression::Terminal(prop) => {
            println!(
                "prop: {:?} symbolic_domains: {:?} variable set: {:?}",
                prop,
                symbolic_domains.keys(),
                bdd_variable_set.variables()
            );
            prop_to_bdd(prop.clone(), symbolic_domains, bdd_variable_set)
        }
        Expression::Not(expr) => {
            let bdd = bdd_from_expr(expr, symbolic_domains, bdd_variable_set);
            bdd.not()
        }
        Expression::And(lhs, rhs) => {
            let lhs = bdd_from_expr(lhs, symbolic_domains, bdd_variable_set);
            let rhs = bdd_from_expr(rhs, symbolic_domains, bdd_variable_set);
            lhs.and(&rhs)
        }
        Expression::Or(lhs, rhs) => {
            let lhs = bdd_from_expr(lhs, symbolic_domains, bdd_variable_set);
            let rhs = bdd_from_expr(rhs, symbolic_domains, bdd_variable_set);
            lhs.or(&rhs)
        }
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
// fn prop_to_bdd<D: SymbolicDomain<T>, T>(
fn prop_to_bdd<D: SymbolicDomain<ValueType>>(
    prop: Proposition,
    symbolic_domains: &HashMap<String, D>,
    bdd_variable_set: &mut BddVariableSet,
) -> Bdd {
    println!("prop ci: <{:?}>", prop.ci);
    println!("domains keys: {:?}", symbolic_domains.keys());

    let var = symbolic_domains.get(&prop.ci).unwrap();
    let val = prop.cn;

    println!("calling encode_one on val: {:?}", val);

    match prop.cmp {
        super::expression::CmpOp::Eq => var.encode_one(bdd_variable_set, &(val as u8)),
        super::expression::CmpOp::Neq => var.encode_one(bdd_variable_set, &(val as u8)).not(),
        super::expression::CmpOp::Lt => lt(var, bdd_variable_set, val),
        super::expression::CmpOp::Leq => leq(var, bdd_variable_set, val),
        super::expression::CmpOp::Gt => leq(var, bdd_variable_set, val).not(),
        super::expression::CmpOp::Geq => lt(var, bdd_variable_set, val).not(),
    }
}

// fn lt<D: SymbolicDomain<T>, T>(
fn lt<D: SymbolicDomain<ValueType>>(
    symbolic_domain: &D,
    bdd_variable_set: &mut BddVariableSet,
    lower_than_this: u16,
) -> Bdd {
    let mut bdd = symbolic_domain.empty_collection(bdd_variable_set);

    (0..lower_than_this).for_each(|i| {
        let bdd_i = symbolic_domain.encode_one(bdd_variable_set, &(i as ValueType));
        bdd = bdd.or(&bdd_i);
    });

    bdd
}

// fn leq<D: SymbolicDomain<T>, T>(
fn leq<D: SymbolicDomain<ValueType>>(
    symbolic_domain: &D,
    bdd_variable_set: &mut BddVariableSet,
    lower_or_same_as_this: u16,
) -> Bdd {
    let mut bdd = symbolic_domain.empty_collection(bdd_variable_set);

    (0..(lower_or_same_as_this + 1)).for_each(|i| {
        let bdd_i = symbolic_domain.encode_one(bdd_variable_set, &(i as ValueType));
        bdd = bdd.or(&bdd_i);
    });

    bdd
}
