use biodivine_lib_bdd::{Bdd, BddPartialValuation, BddValuation};

use crate::{SymbolicDomain, UpdateFnBdd};

// todo want to have a trait abstracting functions of this over different domains
// todo also this should have the domain type as a type parameter
pub struct UpdateFnCompiled {
    pub output_max_value: u8, // this will be encoded in the length of the bdd vec vvv
    // will be used to answer the question:
    // for this (given) valuation, what is the i-th "bit" of the output?
    // i-th bdd answers this for the i-th bit
    pub bit_answering_bdds: Vec<Bdd>,
}

impl From<UpdateFnBdd> for UpdateFnCompiled {
    fn from(update_fn: UpdateFnBdd) -> Self {
        let mut prev = &update_fn.bdd_variable_set.mk_true();
        let mut mutually_exclusive_terms = Vec::<Bdd>::new();

        for (_term_res, term_bdd) in &update_fn.terms {
            let mutually_exclusive_bdd = term_bdd.and(&prev.not());
            mutually_exclusive_terms.push(mutually_exclusive_bdd);
            prev = mutually_exclusive_terms.last().unwrap();
        }

        mutually_exclusive_terms.push(prev.not()); // default value

        // todo abstract the building of this into a function
        // todo also make it functionally not this garbage
        // todo test they are mutually exclusive -> more motivation to move it into a function
        let mutually_exclusive_terms = mutually_exclusive_terms;
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

        Self::new(output_max_value, bit_answering_bdds)
    }
}

impl UpdateFnCompiled {
    // intentionaly private; should only be instantiated through From<UpdateFn>
    fn new(output_max_value: u8, bit_answering_bdds: Vec<Bdd>) -> Self {
        Self {
            output_max_value,
            bit_answering_bdds,
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
    #[test]
    fn test_update_fn_compiled() {
        todo!()
    }
}
