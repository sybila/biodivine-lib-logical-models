use biodivine_lib_bdd::{Bdd, BddPartialValuation};

use crate::{SymbolicDomain, UpdateFnBdd, UpdateFnBdd_};

type ValueType = u8;

// todo this notation is kinda weird; D should already carry info about what T is
// todo for example for UnaryIntegerDomain, T is u8, but i have to specify it like
// todo UpdateFnCompiled::<UnaryIntegerDomain, u8>::from(update_fn_bdd)
// pub struct UpdateFnCompiled_<D: SymbolicDomain<T>, T> {
pub struct UpdateFnCompiled_<D: SymbolicDomain<ValueType>> {
    // phantom: std::marker::PhantomData<(D, T)>,
    pub output_max_value: ValueType, // todo do i need this here? not sufficient just in the compiling method?
    pub bit_answering_bdds: Vec<Bdd>,
    pub named_symbolic_domains: std::collections::HashMap<String, D>,
}

// todo directly from UpdateFn (not UpdateFnBdd)
// impl<D: SymbolicDomain<T>, T> From<UpdateFnBdd_<lol idk something>> for UpdateFnCompiled_<D, T> {
impl<D: SymbolicDomain<ValueType>> From<UpdateFnBdd_<D>> for UpdateFnCompiled_<D> {
    fn from(update_fn_bdd: UpdateFnBdd_<D>) -> Self {
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

        inspect_outputs_numeric_and_bitwise(outputs, matrix);

        panic!("todo");

        let mut bit_answering_bdds = Vec::<Bdd>::new();
        for bit_idx in 0..matrix[0].len() {
            let mut bit_answering_bdd = update_fn_bdd.bdd_variable_set.mk_false();
            for row_idx in 0..matrix.len() {
                if matrix[row_idx][bit_idx] {
                    bit_answering_bdd = bit_answering_bdd.or(&mutually_exclusive_terms[row_idx]);
                }
            }

            let dot = bit_answering_bdd.to_dot_string(&update_fn_bdd.bdd_variable_set, false);
            bit_answering_bdds.push(bit_answering_bdd);
        }

        let output_max_value = todo!();
    }
}

// fn inspect_outputs_numeric_and_bitwise(numeric_and_bitwise: Vec<(u8, Vec<bool>)>) {}
fn inspect_outputs_numeric_and_bitwise(numeric_outputs: Vec<u8>, bit_outputs: Vec<Vec<bool>>) {
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

fn panics() -> ! {
    panic!("this should not happen");
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

#[cfg(test)]
mod tests {
    use biodivine_lib_bdd::{BddPartialValuation, BddVariableSetBuilder};

    use crate::{
        get_test_update_fn, prototype::update_fn_compiled_modular::UpdateFnCompiled_,
        SymbolicDomain, UnaryIntegerDomain, UpdateFnBdd_,
    };

    #[derive(Clone)]
    struct FakeDomain;
    impl SymbolicDomain<u8> for FakeDomain {
        fn new(
            builder: &mut biodivine_lib_bdd::BddVariableSetBuilder,
            name: &str,
            max_value: u8,
        ) -> Self {
            let variables = (0..max_value)
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
            println!("printing variables");
            println!("var len: {}", _variables.variables().len());
            _variables.variables().iter().for_each(|var| {
                println!("variable: {:?}", var);
            });

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
        let update_fn_bdd: UpdateFnBdd_<UnaryIntegerDomain> = update_fn.into();
        let compiled = UpdateFnCompiled_::<UnaryIntegerDomain>::from(update_fn_bdd);
    }

    #[test]
    fn test_fake() {
        let update_fn = get_test_update_fn();
        let update_fn_bdd: UpdateFnBdd_<FakeDomain> = update_fn.into();
        let compiled = UpdateFnCompiled_::<FakeDomain>::from(update_fn_bdd);
    }
}
