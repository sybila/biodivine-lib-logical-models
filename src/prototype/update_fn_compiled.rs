use std::collections::HashMap;

use biodivine_lib_bdd::{Bdd, BddPartialValuation, BddValuation};

use crate::{SymbolicDomain, UnaryIntegerDomain, UpdateFnBdd};

// todo want to have a trait abstracting functions of this over different domains
// todo also this should have the domain type as a type parameter
pub struct UpdateFnCompiled {
    pub output_max_value: u8, // this will be encoded in the length of the bdd vec vvv
    // will be used to answer the question:
    // for this (given) valuation, what is the i-th "bit" of the output?
    // i-th bdd answers this for the i-th bit
    pub bit_answering_bdds: Vec<Bdd>,
    pub named_symbolic_domains: HashMap<String, UnaryIntegerDomain>,
}

impl From<UpdateFnBdd> for UpdateFnCompiled {
    fn from(update_fn: UpdateFnBdd) -> Self {
        // todo test they are mutually exclusive -> more motivation to move it into a function
        // todo there is a possibility to play with the fact that ther might be some terms unreachable
        // todo  those would be indicated by the first bit_answering_bdd being const false
        // todo  could use this to uptimize this (but that is lost once we convert it to bit_answering_bdds)
        // todo  or could use this to give feedback to the user that some cases are unreachable
        let mutually_exclusive_terms = to_mutually_exclusive_and_default(
            update_fn
                .terms
                .iter()
                .map(|(_output, term_bdd)| term_bdd.clone())
                .collect(),
        );

        println!("mutually_exclusive_terms: {:?}", mutually_exclusive_terms);

        let outputs = update_fn
            .terms
            .iter()
            .map(|(output, _)| *output)
            .chain(std::iter::once(update_fn.default))
            .collect::<Vec<_>>();

        println!("outputs: {:?}", outputs);

        let matrix = outputs
            .iter()
            .map(|numeric_output| {
                let mut bit_storage = BddPartialValuation::empty();
                update_fn
                    .result_domain
                    .encode_bits(&mut bit_storage, numeric_output);
                bit_storage
                    .to_values()
                    .into_iter()
                    .map(|(_, bit)| bit)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        println!("matrix: {:?}", matrix);

        let mut bit_answering_bdds = Vec::<Bdd>::new();
        for bit_idx in 0..matrix[0].len() {
            let mut bit_answering_bdd = update_fn.bdd_variable_set.mk_false();
            for row_idx in 0..matrix.len() {
                if matrix[row_idx][bit_idx] {
                    bit_answering_bdd = bit_answering_bdd.or(&mutually_exclusive_terms[row_idx]);
                }
            }

            let dot = bit_answering_bdd.to_dot_string(&update_fn.bdd_variable_set, false);
            println!("dot of index {}: {}", bit_idx, dot);

            bit_answering_bdds.push(bit_answering_bdd);
        }

        println!("bit_answering_bdds: {:?}", bit_answering_bdds);

        bit_answering_bdds
            .iter()
            .enumerate()
            .for_each(|(idx, bdd)| {
                println!("bdd at index {}: {}", idx, bdd);
            });

        matrix.iter().for_each(|row| {
            println!(
                "row: {:?}",
                row.iter()
                    .map(|bit| if *bit { 1 } else { 0 })
                    .collect::<Vec<_>>()
            );
        });

        let output_max_value = matrix[0].len() as u8; // todo get this more elegantly

        Self::new(
            output_max_value,
            bit_answering_bdds,
            update_fn.named_symbolic_domains,
        )
    }
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

impl UpdateFnCompiled {
    // intentionaly private; should only be instantiated through From<UpdateFn>
    fn new(
        output_max_value: u8,
        bit_answering_bdds: Vec<Bdd>,
        named_symbolic_domains: HashMap<String, UnaryIntegerDomain>,
    ) -> Self {
        Self {
            output_max_value,
            bit_answering_bdds,
            named_symbolic_domains,
        }
    }

    pub fn get_result_ith_bit(&self, bit_idx: usize, valuation: &BddValuation) -> bool {
        self.bit_answering_bdds[bit_idx].eval_in(valuation)
    }

    pub fn get_result_bits(&self, valuation: &BddValuation) -> Vec<bool> {
        self.bit_answering_bdds
            .iter()
            .map(|bdd| bdd.eval_in(valuation))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{SymbolicDomain, UpdateFn, UpdateFnBdd, UpdateFnCompiled};

    #[test]
    fn test_update_fn_compiled() {
        let update_fn = get_update_fn();
        let bdd_update_fn: UpdateFnBdd = update_fn.into();
        // todo yeah this should be accessible from compiled as well
        let mut valuation = bdd_update_fn.get_default_valuation_but_partial();
        let bdd_update_fn_compiled: UpdateFnCompiled = bdd_update_fn.into();

        let var_domain = bdd_update_fn_compiled
            .named_symbolic_domains
            .get("renamed")
            .unwrap();

        var_domain.encode_bits(&mut valuation, &1);
        println!("valuation: {:?}", valuation);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );

        var_domain.encode_bits(&mut valuation, &2);
        println!("valuation: {:?}", valuation);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );

        var_domain.encode_bits(&mut valuation, &3);
        println!("valuation: {:?}", valuation);
        println!(
            "result: {:?}",
            bdd_update_fn_compiled.get_result_bits(&valuation.clone().try_into().unwrap())
        );
    }

    fn get_update_fn() -> UpdateFn {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open("data/update_fn_test.sbml").expect("cannot open file");
        let file = BufReader::new(file);

        let mut xml = xml::reader::EventReader::new(file);

        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    if name.local_name == "transition" {
                        let update_fn = UpdateFn::try_from_xml(&mut xml);
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
