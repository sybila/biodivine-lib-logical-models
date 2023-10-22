#![allow(dead_code)]

// todo how to work with the "variables" that are not mentioned in the listOfTransitions?

use std::{collections::HashMap, io::BufRead};

use biodivine_lib_bdd::{BddPartialValuation, BddValuation, BddVariable, BddVariableSetBuilder};

use crate::{SymbolicDomain, UpdateFn, UpdateFnBdd, VariableUpdateFnCompiled, XmlReader};

#[derive(Debug)]
pub struct SystemUpdateFn<D: SymbolicDomain<T>, T> {
    pub update_fns: HashMap<String, VariableUpdateFnCompiled<D, T>>,
    pub named_symbolic_domains: HashMap<String, D>,
}

impl<D: SymbolicDomain<u8>> SystemUpdateFn<D, u8> {
    /// expects the xml reader to be at the start of the <listOfTransitions> element
    pub fn try_from_xml<XR: XmlReader<BR>, BR: BufRead>(
        xml: &mut XR,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let var_names_and_upd_fns = load_all_update_fns(xml)?;
        let ctx = vars_and_their_max_values(&var_names_and_upd_fns);

        // todo currently, we have no way of adding those variables, that do not have their VariableUpdateFn
        // todo  (ie their qual:transition in the xml) into the named_symbolic_domains, even tho they migh
        // todo  be used as inputs to some functions, causing panic
        let mut bdd_variable_set_builder = BddVariableSetBuilder::new();
        let named_symbolic_domains = ctx
            .into_iter()
            .map(|(name, max_value)| {
                (
                    name.clone(),
                    D::new(&mut bdd_variable_set_builder, &name, max_value.to_owned()),
                )
            })
            .collect::<HashMap<_, _>>();
        let variable_set = bdd_variable_set_builder.build();

        let update_fns = var_names_and_upd_fns
            .into_values()
            .map(|update_fn| {
                (
                    update_fn.target_var_name.clone(),
                    UpdateFnBdd::from_update_fn(update_fn, &variable_set, &named_symbolic_domains)
                        .into(),
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(Self {
            update_fns,
            named_symbolic_domains,
        })
    }

    /// returns valuation inicialized so that all the symbolic values are = 0
    pub fn get_default_partial_valuation(&self) -> BddPartialValuation {
        self.named_symbolic_domains.values().fold(
            BddPartialValuation::empty(),
            |mut acc, domain| {
                println!("line 72");
                domain.encode_bits(&mut acc, &0);
                acc
            },
        )
    }

    /// panics if this system does not contain variable of `sym_var_name` name
    pub fn get_result_bits(
        &self,
        sym_var_name: &str,
        valuation: &BddValuation,
    ) -> Vec<(bool, BddVariable)> {
        self.update_fns
            .get(sym_var_name)
            .unwrap()
            .get_result_bits(valuation)
    }

    pub fn get_successor_under_given_variable_update_fn(
        &self,
        variable_name: &str,
        valuation: &BddValuation,
    ) -> BddValuation {
        let bits = self.get_result_bits(variable_name, valuation);
        let mut new_valuation = valuation.clone();

        bits.into_iter().for_each(|(bool, var)| {
            new_valuation.set_value(var, bool);
        });

        new_valuation
    }

    pub fn get_successors(&self, valuation: &BddValuation) -> Vec<BddValuation> {
        self.named_symbolic_domains
            .keys()
            .map(|var_name| self.get_successor_under_given_variable_update_fn(var_name, valuation))
            .collect()
    }
}

/// expects the xml reader to be at the start of the <listOfTransitions> element
fn load_all_update_fns<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    // todo generic
) -> Result<HashMap<String, UpdateFn<u8>>, Box<dyn std::error::Error>> {
    Ok(crate::process_list(
        "listOfTransitions",
        "transition",
        |xml, _unused_opening_tag| UpdateFn::<u8>::try_from_xml(xml),
        xml,
    )?
    // todo this might not be the smartest nor useful; the name is already in the fn
    //  but this will allow us to access the appropriate fn quickly
    .into_iter()
    .map(|upd_fn| (upd_fn.target_var_name.clone(), upd_fn))
    .collect())
}

// todo this might not be the best way; it cannot detect that some values are unreachable;
// todo  for example:
//     term0: output = 1 if (const true)
//     default: output = 9999
// todo this will be detected as max value = 9999, but in reality it is 1
// todo; -> warnings abt unreachable terms important during conversion to CompiledUpdateFn
/// returns a map of variable names and their max values
fn vars_and_their_max_values(
    vars_and_their_upd_fns: &HashMap<String, UpdateFn<u8>>,
) -> HashMap<String, u8> {
    // todo this will be sufficient once there is a guarantee that the variables
    // todo  are not used (in comparasions) with values larger than their max_value
    // todo  (where max_value is determined by the transition function of that variable)
    // vars_and_their_upd_fns
    //     .iter()
    //     .map(|(var_name, upd_fn)| {
    //         (
    //             var_name.clone(),
    //             upd_fn
    //                 .terms
    //                 .iter()
    //                 .map(|(val, _)| val)
    //                 .chain(std::iter::once(&upd_fn.default))
    //                 .max() // todo this is not enough
    //                 .unwrap()
    //                 .to_owned(),
    //         )
    //     })
    //     .collect()

    vars_and_their_upd_fns
        .iter()
        .map(|(var_name, _update_fn)| {
            (
                var_name.clone(),
                get_max_val_of_var_in_all_transitions_including_their_own_and_detect_where_compared_with_larger_than_possible(
                    var_name,
                    vars_and_their_upd_fns,
                ),
            )
        })
        .collect()
}

// todo this should be rewritten so that first, the maximum of all the variables is computed
// todo (which can be done in O(n) with respect to the count of transitions using hashmap<var_name, max_so_far>)
// todo and only then will the results be used
fn get_max_val_of_var_in_all_transitions_including_their_own(
    var_name: &str,
    update_fns: &HashMap<String, UpdateFn<u8>>,
) -> u8 {
    let max_in_its_update_fn = update_fns
        .get(var_name)
        .unwrap()
        .terms
        .iter()
        .map(|(val, _)| val)
        .chain(std::iter::once(&update_fns.get(var_name).unwrap().default)) // add the value of default term
        .max() // todo this is not enough
        .unwrap() // safe unwrap; at least default terms value always present
        .to_owned();

    let max_in_terms = update_fns
        .values()
        .filter_map(|update_fn| {
            update_fn
                .terms
                .iter()
                .filter_map(|term| term.1.highest_value_used_with_variable(var_name))
                .max()
        })
        .max();

    match max_in_terms {
        None => max_in_its_update_fn,
        Some(max_in_terms) => std::cmp::max(max_in_its_update_fn, max_in_terms),
    }
}

fn get_max_val_of_var_in_all_transitions_including_their_own_and_detect_where_compared_with_larger_than_possible(
    var_name: &str,
    update_fns: &HashMap<String, UpdateFn<u8>>,
) -> u8 {
    let max_in_its_update_fn = update_fns
        .get(var_name)
        .unwrap()
        .terms
        .iter()
        .map(|(val, _)| val)
        .chain(std::iter::once(&update_fns.get(var_name).unwrap().default)) // add the value of default term
        .max()
        .unwrap() // safe unwrap; at least the default terms value always present
        .to_owned();

    let max_in_terms = update_fns
        .values()
        .filter_map(|update_fn| {
            update_fn
                .terms
                .iter()
                .filter_map(|term| {
                    term.1
                        .highest_value_used_with_variable_detect_higher_than_exected(
                            var_name,
                            max_in_its_update_fn,
                        )
                })
                .max()
        })
        .max();

    match max_in_terms {
        None => max_in_its_update_fn,
        Some(max_in_terms) => std::cmp::max(max_in_its_update_fn, max_in_terms),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        symbolic_domain::{BinaryIntegerDomain, GrayCodeIntegerDomain, PetriNetIntegerDomain},
        SymbolicDomain, UnaryIntegerDomain, XmlReader,
    };

    // use std:io::{BufRead, BufReader}

    use std::io::BufRead;
    use std::io::BufReader;

    use super::SystemUpdateFn;

    #[test]
    fn test() {
        let file = std::fs::File::open("data/dataset.sbml").expect("cannot open file");
        let br = std::io::BufReader::new(file);

        let reader = xml::reader::EventReader::new(br);
        let mut reader = crate::LoudReader::new(reader); // uncomment to see how xml is loaded

        crate::find_start_of(&mut reader, "listOfTransitions").expect("cannot find start of list");
        let system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
            super::SystemUpdateFn::try_from_xml(&mut reader).unwrap();

        let mut valuation = system_update_fn.get_default_partial_valuation();
        let some_domain = system_update_fn
            .named_symbolic_domains
            .get("todo some existing name")
            .unwrap();

        println!("line 184");
        some_domain.encode_bits(&mut valuation, &1);
    }

    #[test]
    fn test_bigger() {
        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open("data/bigger.sbml").unwrap(),
        ));

        crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find start of list");

        let system_update_fn: SystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
            super::SystemUpdateFn::try_from_xml(&mut xml).expect("cannot load system update fn");

        println!("system_update_fn: {:?}", system_update_fn);

        let mut valuation = system_update_fn.get_default_partial_valuation();
        let domain = system_update_fn.named_symbolic_domains.get("ORI").unwrap();
        domain.encode_bits(&mut valuation, &1);

        let succs = system_update_fn.get_successors(&valuation.try_into().unwrap());
        println!("succs: {:?}", succs);
    }

    #[test]
    fn test_all_bigger() {
        std::fs::read_dir("data/large")
            .expect("could not read dir")
            .for_each(|dirent| {
                println!("dirent = {:?}", dirent);
                let dirent = dirent.expect("could not read file");

                let xml = xml::reader::EventReader::new(std::io::BufReader::new(
                    std::fs::File::open(dirent.path()).unwrap(),
                ));

                let mut counting = crate::CountingReader::new(xml);

                crate::find_start_of(&mut counting, "listOfTransitions")
                    .expect("could not find list");

                let start = counting.curr_line;

                let _system_update_fn: SystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                    super::SystemUpdateFn::try_from_xml(&mut counting)
                        .expect("cannot load system update fn");

                println!("file size = {:?}", counting.curr_line);
                println!(
                    "just the transitions list = {:?}",
                    counting.curr_line - start
                );

                // println!("system_update_fn: {:?}", system_update_fn);
            })
    }

    #[test]
    fn test_all_bigger_with_debug_xml_reader() {
        std::fs::read_dir("data/faulty")
            .expect("could not read dir")
            .for_each(|dirent| {
                println!("dirent = {:?}", dirent);
                let dirent = dirent.expect("could not read file");

                let xml = xml::reader::EventReader::new(std::io::BufReader::new(
                    std::fs::File::open(dirent.path()).unwrap(),
                ));

                let mut counting = crate::CountingReader::new(xml);

                crate::find_start_of(&mut counting, "listOfTransitions")
                    .expect("could not find list");

                let all_update_fns = super::load_all_update_fns(&mut counting)
                    .expect("could not even load the damn thing");

                let xml = xml::reader::EventReader::new(std::io::BufReader::new(
                    std::fs::File::open(dirent.path()).unwrap(),
                ));
                let mut debug_xml = crate::DebuggingReader::new(xml, &all_update_fns, true, true);

                // while let Ok(_) = debug_xml.next() {}

                loop {
                    if let xml::reader::XmlEvent::EndDocument = debug_xml.next().unwrap() {
                        break;
                    }
                }

                // let _system_update_fn: SystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                //     super::SystemUpdateFn::try_from_xml(&mut counting)
                //         .expect("cannot load system update fn");

                // println!("file size = {:?}", counting.curr_line);
                // println!(
                //     "just the transitions list = {:?}",
                //     counting.curr_line - start
                // );

                // println!("system_update_fn: {:?}", system_update_fn);
            })
    }

    #[test]
    fn test_on_test_data() {
        let mut reader = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open("data/update_fn_test.sbml").unwrap(),
        ));

        crate::find_start_of(&mut reader, "listOfTransitions").expect("cannot find start of list");
        let system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
            super::SystemUpdateFn::try_from_xml(&mut reader).unwrap();

        let mut valuation = system_update_fn.get_default_partial_valuation();
        let domain_renamed = system_update_fn
            .named_symbolic_domains
            .get("renamed")
            .unwrap();
        println!("line 217");
        domain_renamed.encode_bits(&mut valuation, &6);

        let res =
            system_update_fn.get_result_bits("renamed", &valuation.clone().try_into().unwrap());

        let mut new_valuation = valuation.clone();
        res.into_iter().for_each(|(bool, var)| {
            new_valuation.set_value(var, bool);
        });

        println!("valuation: {:?}", valuation);
        println!("new_valuation: {:?}", new_valuation);

        let successors = system_update_fn.get_successors(&valuation.clone().try_into().unwrap());
        println!("successors: {:?}", successors);
    }
}
