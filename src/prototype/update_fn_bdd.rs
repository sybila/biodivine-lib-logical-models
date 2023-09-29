use crate::{Expression, SymbolicDomain, UnaryIntegerDomain, UpdateFn};

use std::collections::HashMap;

use biodivine_lib_bdd::{Bdd, BddValuation, BddVariableSet, BddVariableSetBuilder};

use super::expression::Proposition;

// todo currently do not know how to determine the max value of a variable; hardcoding it for now; should be extracted from the xml/UpdateFn
const HARD_CODED_MAX_VAR_VALUE: u8 = 2;

/// describes, how single variable is updated
/// set of UpdateFnBdds is used to describe the dynamics of the whole system
pub struct UpdateFnBdd {
    pub target_var_name: String,
    pub terms: Vec<(u16, Bdd)>,
    pub named_symbolic_domains: HashMap<String, UnaryIntegerDomain>,
    // todo might make sense to compile the different term bdds so that n+1th is (not n) && (whatever term)
    // todo -> that way we could even run the computations parallelly & return the value corresponding to the single true output
    // todo but that might not make sense; likely computing bits rather than a single value
    pub default: u16, // the one that is used when no condition is met;
}

// todo UpdateFn should be made obsolete, it is just an intermediate representation of what should eventually be UpdateFnBdd
impl From<UpdateFn> for UpdateFnBdd {
    fn from(source: UpdateFn) -> Self {
        let mut bdd_variable_set_builder = BddVariableSetBuilder::new();

        let named_symbolic_domains = source
            .input_vars_names
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    UnaryIntegerDomain::new(
                        &mut bdd_variable_set_builder,
                        name,
                        HARD_CODED_MAX_VAR_VALUE,
                    ),
                )
            })
            .collect();

        let mut bdd_variable_set = bdd_variable_set_builder.build();
        let terms = source
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

        Self {
            target_var_name: source.target_var_name,
            terms,
            named_symbolic_domains,
            default: source.default,
        }
    }
}

fn bdd_from_expr(
    expr: &Expression,
    symbolic_domains: &HashMap<String, UnaryIntegerDomain>,
    bdd_variable_set: &mut BddVariableSet,
) -> Bdd {
    match expr {
        // prop_to_bdd is the important thing here;
        // the rest is just recursion & calling the right bdd methods
        Expression::Terminal(prop) => prop_to_bdd(prop.clone(), symbolic_domains, bdd_variable_set),
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

impl UpdateFnBdd {
    /// for given valuation of input variables, returns the value of the output variable according to the update function
    /// todo should probably accept valuations of the symbolic variables
    /// todo so that user is abstracted from having to specify vector of bools
    /// todo and instead can just specify the values of symbolic variables
    /// todo for now, i know what is the underlying representation of the symbolic variables
    /// todo -> good enough for testing
    pub fn eval_in(&self, valuation: &BddValuation) -> u16 {
        self.terms
            .iter()
            .find(|(_, bdd)| bdd.eval_in(valuation))
            .map(|(val, _)| *val)
            .unwrap_or(self.default)
    }
}

// todo this should be applied to each term directly while loading the xml; no need to even have the intermediate representation
fn prop_to_bdd(
    prop: Proposition,
    symbolic_domains: &HashMap<String, UnaryIntegerDomain>,
    bdd_variable_set: &mut BddVariableSet,
) -> Bdd {
    println!("prop ci: <{:?}>", prop.ci);
    println!("domains keys: {:?}", symbolic_domains.keys());

    let var = symbolic_domains.get(&prop.ci).unwrap();
    let val = prop.cn;

    match prop.cmp {
        super::expression::CmpOp::Eq => var.encode_one(bdd_variable_set, &(val as u8)),
        super::expression::CmpOp::Neq => var.encode_one(bdd_variable_set, &(val as u8)).not(),
        super::expression::CmpOp::Lt => lt(var, bdd_variable_set, val),
        super::expression::CmpOp::Leq => leq(var, bdd_variable_set, val),
        super::expression::CmpOp::Gt => leq(var, bdd_variable_set, val).not(),
        super::expression::CmpOp::Geq => lt(var, bdd_variable_set, val).not(),
    }
}

fn lt(
    symbolic_domain: &UnaryIntegerDomain,
    bdd_variable_set: &mut BddVariableSet,
    lower_than_this: u16,
) -> Bdd {
    let mut bdd = symbolic_domain.empty_collection(bdd_variable_set);

    (0..lower_than_this).for_each(|i| {
        let bdd_i = symbolic_domain.encode_one(bdd_variable_set, &(i as u8));
        bdd = bdd.or(&bdd_i);
    });

    bdd
}

fn leq(
    symbolic_domain: &UnaryIntegerDomain,
    bdd_variable_set: &mut BddVariableSet,
    lower_or_same_as_this: u16,
) -> Bdd {
    let mut bdd = symbolic_domain.empty_collection(bdd_variable_set);

    (0..(lower_or_same_as_this + 1)).for_each(|i| {
        let bdd_i = symbolic_domain.encode_one(bdd_variable_set, &(i as u8));
        bdd = bdd.or(&bdd_i);
    });

    bdd
}

mod tests {
    use biodivine_lib_bdd::{BddPartialValuation, BddValuation};

    use crate::{SymbolicDomain, UpdateFnBdd};

    #[test]
    fn test_update_fn_result() {
        let update_fn: UpdateFnBdd = get_update_fn().into();

        // update_fn.terms.iter().for_each(|(val, bdd)| {
        //     println!("val: {}, bdd: {:?}", val, bdd);
        // });

        println!(
            "@@@@@@@@@@@@@@@@@@@@@@@update fn terms len: {:?}",
            update_fn.terms.len()
        );

        // let valuation = BddValuation::new(vec![false, false]);

        // let res = update_fn.terms[0].1.eval_in(&valuation);
        // println!(
        //     "res for term at idx 0 with valuation {}: {}",
        //     valuation, res
        // );
        // let res = update_fn.terms[1].1.eval_in(&valuation);
        // println!(
        //     "res for term at idx 1 with valuation {}: {}",
        //     valuation, res
        // );
        // let res = update_fn.terms[2].1.eval_in(&valuation);
        // println!(
        //     "res for term at idx 2 with valuation {}: {}",
        //     valuation, res
        // );
        // let res = update_fn.terms[3].1.eval_in(&valuation);
        // println!(
        //     "res for term at idx 3 with valuation {}: {}",
        //     valuation, res
        // );

        // println!(
        //     "eval for false, false aka 0: {}",
        //     update_fn.eval_in(&BddValuation::new(vec![false, false]))
        // );
        // println!(
        //     "eval for false, true aka 1: {}",
        //     update_fn.eval_in(&BddValuation::new(vec![false, true]))
        // );
        // println!(
        //     "eval for true, false aka 2: {}",
        //     update_fn.eval_in(&BddValuation::new(vec![true, false]))
        // );
        // println!(
        //     "eval for true, true aka 3: {}",
        //     update_fn.eval_in(&BddValuation::new(vec![true, true]))
        // );

        let valuation = BddValuation::new(vec![false, false]);
        let accepted = update_fn.terms[0].1.eval_in(&valuation);
        println!("accepted for valuation {}?: {}", valuation, accepted);

        let valuation = BddValuation::new(vec![false, true]);
        let accepted = update_fn.terms[0].1.eval_in(&valuation);
        println!("accepted for valuation {}?: {}", valuation, accepted);

        let valuation = BddValuation::new(vec![true, false]);
        let accepted = update_fn.terms[0].1.eval_in(&valuation);
        println!("accepted for valuation {}?: {}", valuation, accepted);

        let valuation = BddValuation::new(vec![true, true]);
        let accepted = update_fn.terms[0].1.eval_in(&valuation);
        println!("accepted for valuation {}?: {}", valuation, accepted);

        // partial should provide me with api to get the vector of bools representing the valuation
    }

    #[test]
    pub fn test_update_fn() {
        let update_fn = get_update_fn();
        println!("update fn: {:?}", update_fn);

        let update_fn_bdd: UpdateFnBdd = update_fn.into();

        let domain = update_fn_bdd.named_symbolic_domains.get("Mdm2nuc").unwrap();

        let mut valuation = BddPartialValuation::empty();

        domain.encode_bits(&mut valuation, &1);

        let bdd = update_fn_bdd.terms[0].1.clone();

        println!(
            "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@var is represented by {:?} in bdd",
            domain.symbolic_variables()
        );

        let actual_valuation = BddValuation::new({
            let mut vec = vec![false; domain.symbolic_variables().len()];
            vec[0] = true;
            vec
        });

        let bdd_res = bdd.eval_in(&actual_valuation);

        println!("bdd res: {:?}", bdd_res);
    }

    fn get_update_fn() -> super::UpdateFn {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open("data/dataset.sbml").expect("cannot open file");
        let file = BufReader::new(file);

        let mut xml = xml::reader::EventReader::new(file);

        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    if name.local_name == "transition" {
                        let update_fn = super::UpdateFn::try_from_xml(&mut xml);
                        return update_fn.unwrap();
                    }
                }
                Ok(xml::reader::XmlEvent::EndElement { .. }) => continue,
                Ok(xml::reader::XmlEvent::EndDocument) => panic!(),
                Err(_) => panic!(),
                _ => continue,
            }
        }
    }
}
