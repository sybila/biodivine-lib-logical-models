use std::collections::HashMap;

use biodivine_lib_bdd::{Bdd, BddPartialValuation, BddValuation, BddVariable};

use crate::{SymbolicDomain, UpdateFnBdd};

// type ValueType = u8;

// todo this notation is kinda weird; D should already carry info about what T is
// todo for example for UnaryIntegerDomain, T is u8, but i have to specify it like
// todo UpdateFnCompiled::<UnaryIntegerDomain, u8>::from(update_fn_bdd)
/// describes, how **single** variable from the system is updated based on the valuation of the system
pub struct VariableUpdateFnCompiled<D: SymbolicDomain<T>, T> {
    penis: std::marker::PhantomData<T>,
    pub bit_answering_bdds: Vec<(Bdd, BddVariable)>,
    pub named_symbolic_domains: HashMap<String, D>,
}

// todo directly from UpdateFn (not UpdateFnBdd)
impl<D: SymbolicDomain<u8>> From<UpdateFnBdd<D>> for VariableUpdateFnCompiled<D, u8> {
    fn from(update_fn_bdd: UpdateFnBdd<D>) -> Self {
        let mutually_exclusive_terms = to_mutually_exclusive_and_default(
            update_fn_bdd
                .terms
                .iter()
                .map(|(_output, term_bdd)| term_bdd.clone())
                .collect(),
        );

        let outputs = update_fn_bdd
            .terms
            .iter()
            .map(|(output, _term_bdd)| *output)
            .chain(std::iter::once(update_fn_bdd.default)) // the output for the last, default term
            .collect::<Vec<_>>();

        let matrix = outputs
            .iter()
            .map(|numeric_output| {
                let mut bit_storage = BddPartialValuation::empty();
                update_fn_bdd
                    .result_domain
                    .encode_bits(&mut bit_storage, numeric_output);
                bit_storage
                    .to_values()
                    .into_iter()
                    .map(|(_, bit)| bit)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // debug printing
        // _inspect_outputs_numeric_and_bitwise(outputs, matrix.clone());

        let mut bit_answering_bdds = Vec::<Bdd>::new();
        for bit_idx in 0..matrix[0].len() {
            let mut bit_answering_bdd = update_fn_bdd.bdd_variable_set.mk_false();
            for row_idx in 0..matrix.len() {
                if matrix[row_idx][bit_idx] {
                    bit_answering_bdd = bit_answering_bdd.or(&mutually_exclusive_terms[row_idx]);
                }
            }

            bit_answering_bdds.push(bit_answering_bdd);
        }

        let bit_answering_bdds = bit_answering_bdds
            .into_iter()
            .zip(update_fn_bdd.result_domain.symbolic_variables())
            .collect::<Vec<_>>();

        Self::new(bit_answering_bdds, update_fn_bdd.named_symbolic_domains)
    }
}

fn _inspect_outputs_numeric_and_bitwise(numeric_outputs: Vec<u8>, bit_outputs: Vec<Vec<bool>>) {
    if numeric_outputs.len() != bit_outputs.len() {
        panic!("lengths of numeric and bit outputs do not match");
    }

    numeric_outputs
        .iter()
        .zip(bit_outputs)
        .for_each(|(num, bits)| {
            println!("{}: {:?}", num, bits);
        });
}

/// converts a succession of bdds into a succession of bdds, such that ith bdd in the result
/// is true for given valuation iff ith bdd in the input is true and all bdds before it are false (for that valuation)
/// tldr basically succession of guards
/// adds one more bdd at the end, which is true iff all bdds in the input are false (for given valuation)
/// todo maybe rewrite this to use fold, but this might be more readable
fn to_mutually_exclusive_and_default(bdd_succession: Vec<Bdd>) -> Vec<Bdd> {
    if bdd_succession.is_empty() {
        panic!("got empty bdd succession"); // this should not happen; only using this here
    }

    let mut seen = bdd_succession[0].and_not(&bdd_succession[0]); // const false
    let mut mutually_exclusive_terms = Vec::<Bdd>::new();

    for term_bdd in bdd_succession {
        let mutually_exclusive_bdd = term_bdd.and(&seen.not());
        mutually_exclusive_terms.push(mutually_exclusive_bdd);
        seen = seen.or(&term_bdd);
    }

    mutually_exclusive_terms.push(seen.not()); // default value

    mutually_exclusive_terms
}

impl<D: SymbolicDomain<T>, T> VariableUpdateFnCompiled<D, T> {
    // intentionally private; should only be instantiated through From<UpdateFnBdd_>
    fn new(
        bit_answering_bdds: Vec<(Bdd, BddVariable)>,
        named_symbolic_domains: HashMap<String, D>,
    ) -> Self {
        Self {
            penis: std::marker::PhantomData::<T>,
            bit_answering_bdds,
            named_symbolic_domains,
        }
    }

    pub fn get_result_bits(&self, valuation: &BddValuation) -> Vec<(bool, BddVariable)> {
        self.bit_answering_bdds
            .iter()
            .map(|(bdd, variable)| (bdd.eval_in(valuation), variable.to_owned()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use biodivine_lib_bdd::{BddPartialValuation, BddVariableSetBuilder};

    use crate::{
        get_test_update_fn,
        prototype::update_fn_compiled::VariableUpdateFnCompiled,
        symbolic_domain::{BinaryIntegerDomain, GrayCodeIntegerDomain, PetriNetIntegerDomain},
        SymbolicDomain, UnaryIntegerDomain, UpdateFnBdd,
    };

    #[derive(Clone)]
    struct FakeDomain;
    impl SymbolicDomain<u8> for FakeDomain {
        fn new(
            builder: &mut biodivine_lib_bdd::BddVariableSetBuilder,
            name: &str,
            max_value: u8,
        ) -> Self {
            let _ = (0..max_value)
                .map(|it| {
                    let name = format!("{name}_v{}", it + 1);
                    builder.make_variable(name.as_str())
                })
                .collect::<Vec<_>>();

            Self // now see why it is called fake
        }

        fn decode_bits(&self, _bdd_valuation: &biodivine_lib_bdd::BddPartialValuation) -> u8 {
            todo!()
        }

        fn decode_collection(
            &self,
            _variables: &biodivine_lib_bdd::BddVariableSet,
            _collection: &biodivine_lib_bdd::Bdd,
        ) -> Vec<u8> {
            todo!()
        }

        fn decode_one(
            &self,
            _variables: &biodivine_lib_bdd::BddVariableSet,
            _value: &biodivine_lib_bdd::Bdd,
        ) -> u8 {
            todo!()
        }

        fn empty_collection(
            &self,
            _variables: &biodivine_lib_bdd::BddVariableSet,
        ) -> biodivine_lib_bdd::Bdd {
            todo!()
        }

        fn encode_bits(
            &self,
            _bdd_valuation: &mut biodivine_lib_bdd::BddPartialValuation,
            _value: &u8,
        ) {
            let bdd_variable = BddVariableSetBuilder::new().make_variable("lol");
            _bdd_valuation.set_value(bdd_variable, false);
        }

        fn encode_collection(
            &self,
            _variables: &biodivine_lib_bdd::BddVariableSet,
            _collection: &[u8],
        ) -> biodivine_lib_bdd::Bdd {
            todo!()
        }

        fn encode_one(
            &self,
            _variables: &biodivine_lib_bdd::BddVariableSet,
            value: &u8,
        ) -> biodivine_lib_bdd::Bdd {
            let mut valuation = BddPartialValuation::empty();
            self.encode_bits(&mut valuation, value);
            _variables.mk_conjunctive_clause(&valuation)
        }

        fn symbolic_size(&self) -> usize {
            todo!()
        }

        fn symbolic_variables(&self) -> Vec<biodivine_lib_bdd::BddVariable> {
            todo!()
        }

        fn unit_collection(
            &self,
            _variables: &biodivine_lib_bdd::BddVariableSet,
        ) -> biodivine_lib_bdd::Bdd {
            todo!()
        }
    }

    #[test]
    fn test() {
        let update_fn = get_test_update_fn();
        let update_fn_bdd: UpdateFnBdd<UnaryIntegerDomain> = update_fn.into();
        let _compiled = VariableUpdateFnCompiled::<UnaryIntegerDomain, u8>::from(update_fn_bdd);
    }

    #[test]
    fn test_fake() {
        let update_fn = get_test_update_fn();
        let update_fn_bdd: UpdateFnBdd<FakeDomain> = update_fn.into();
        let _compiled = VariableUpdateFnCompiled::<FakeDomain, u8>::from(update_fn_bdd);
    }

    #[test]
    fn test_update_fn_compiled() {
        let update_fn = get_test_update_fn();
        let bdd_update_fn: UpdateFnBdd<GrayCodeIntegerDomain<u8>> = update_fn.into();
        // todo yeah this should be accessible from compiled as well
        let mut valuation = bdd_update_fn.get_default_valuation_but_partial();
        let bdd_update_fn_compiled: VariableUpdateFnCompiled<GrayCodeIntegerDomain<u8>, u8> =
            bdd_update_fn.into();

        let var_domain = bdd_update_fn_compiled
            .named_symbolic_domains
            .get("renamed")
            .unwrap();

        let valuation_value = 0;
        var_domain.encode_bits(&mut valuation, &valuation_value);
        println!("valuation: {:?}", valuation_value);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );

        let valuation_value = 1;
        var_domain.encode_bits(&mut valuation, &valuation_value);
        println!("valuation: {:?}", valuation_value);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );

        let valuation_value = 2;
        var_domain.encode_bits(&mut valuation, &valuation_value);
        println!("valuation: {:?}", valuation_value);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );

        let valuation_value = 3;
        var_domain.encode_bits(&mut valuation, &valuation_value);
        println!("valuation: {:?}", valuation_value);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );

        let valuation_value = 4;
        var_domain.encode_bits(&mut valuation, &valuation_value);
        println!("valuation: {:?}", valuation_value);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );

        let valuation_value = 5;
        var_domain.encode_bits(&mut valuation, &valuation_value);
        println!("valuation: {:?}", valuation_value);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );
    }
}
