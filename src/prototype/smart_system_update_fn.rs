#![allow(dead_code)]

// todo how to work with the "variables" that are not mentioned in the listOfTransitions?

use std::{collections::HashMap, io::BufRead};

use biodivine_lib_bdd::{Bdd, BddPartialValuation, BddVariableSet, BddVariableSetBuilder};
use debug_ignore::DebugIgnore;

use crate::{
    SymbolicDomain, SymbolicTransitionFn, UpdateFn, UpdateFnBdd, VariableUpdateFnCompiled,
    XmlReader,
};

#[derive(Debug)]
struct SmartSystemUpdateFn<D: SymbolicDomain<T>, T> {
    pub update_fns: HashMap<String, SymbolicTransitionFn<D, T>>,
    pub named_symbolic_domains: HashMap<String, D>,
    bdd_variable_set: DebugIgnore<BddVariableSet>,
}

impl<D: SymbolicDomain<u8>> SmartSystemUpdateFn<D, u8> {
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
            .flat_map(|(name, max_value)| {
                let original_name = name.clone();
                let original = (
                    original_name.clone(),
                    D::new(
                        &mut bdd_variable_set_builder,
                        &original_name,
                        max_value.to_owned(),
                    ),
                );

                let primed_name = format!("{}'", name.clone());
                let primed = (
                    primed_name.clone(),
                    D::new(
                        &mut bdd_variable_set_builder,
                        &primed_name,
                        max_value.to_owned(),
                    ),
                );

                [original, primed].into_iter()
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
            .collect::<HashMap<String, VariableUpdateFnCompiled<D, u8>>>();

        let smart_update_fns = update_fns
            .into_iter()
            // .map(|(target_variable_name, compiled_update_fn)| {
            .map(|tuple| {
                let target_variable_name = tuple.0;
                let compiled_update_fn = tuple.1;
                (
                    target_variable_name.clone(),
                    SymbolicTransitionFn::from_update_fn_compiled(
                        &compiled_update_fn,
                        &variable_set,
                        &target_variable_name,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(Self {
            update_fns: smart_update_fns,
            named_symbolic_domains,
            bdd_variable_set: variable_set.into(),
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

    pub fn transition_under_variable(
        &self,
        transitioned_variable_name: &str,
        current_state: &Bdd,
    ) -> Bdd {
        let used_transition_fn = self
            .update_fns
            .get(transitioned_variable_name)
            .expect("no such variable");
        let states_capable_of_performing_the_transition =
            used_transition_fn.transition_function.and(current_state);

        let target_symbolic_domain = self
            .named_symbolic_domains
            .get(transitioned_variable_name)
            .expect("no such variable");

        let target_symbolic_domain_primed = self
            .named_symbolic_domains
            .get(&format!("{}'", transitioned_variable_name))
            .expect("no such variable");

        let mut acc = states_capable_of_performing_the_transition;
        for bdd_variable in target_symbolic_domain.symbolic_variables() {
            acc = acc.var_exists(bdd_variable);
        }

        for bdd_variable in target_symbolic_domain.symbolic_variables() {
            let bdd_variable_primed = crate::prototype::utils::find_bdd_variables_prime(
                &bdd_variable,
                target_symbolic_domain,
                target_symbolic_domain_primed,
            );

            unsafe {
                acc.rename_variable(bdd_variable_primed, bdd_variable);
            };
        }

        acc
    }

    pub fn get_empty_state_subset(&self) -> Bdd {
        self.bdd_variable_set.0.mk_false()
    }

    pub fn get_whole_state_space_subset(&self) -> Bdd {
        self.bdd_variable_set.0.mk_true()
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
                get_max_val_of_var_in_all_transitions_including_their_own(
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

// fn get_max_val_of_var_in_all_transitions_including_their_own_and_detect_where_compared_with_larger_than_possible(
//     var_name: &str,
//     update_fns: &HashMap<String, UpdateFn<u8>>,
// ) -> u8 {
//     let max_in_its_update_fn = update_fns
//         .get(var_name)
//         .unwrap()
//         .terms
//         .iter()
//         .map(|(val, _)| val)
//         .chain(std::iter::once(&update_fns.get(var_name).unwrap().default)) // add the value of default term
//         .max()
//         .unwrap() // safe unwrap; at least the default terms value always present
//         .to_owned();

//     let max_in_terms = update_fns
//         .values()
//         .filter_map(|update_fn| {
//             update_fn
//                 .terms
//                 .iter()
//                 .filter_map(|term| {
//                     term.1
//                         .highest_value_used_with_variable_detect_higher_than_exected(
//                             var_name,
//                             max_in_its_update_fn,
//                         )
//                 })
//                 .max()
//         })
//         .max();

//     match max_in_terms {
//         None => max_in_its_update_fn,
//         Some(max_in_terms) => std::cmp::max(max_in_its_update_fn, max_in_terms),
//     }
// }

#[cfg(test)]
mod tests {
    use crate::{
        prototype::smart_system_update_fn::{self, SmartSystemUpdateFn},
        symbolic_domain::{BinaryIntegerDomain, GrayCodeIntegerDomain, PetriNetIntegerDomain},
        SymbolicDomain, SystemUpdateFn, UnaryIntegerDomain, XmlReader,
    };

    // use std:io::{BufRead, BufReader}

    use std::io::BufRead;
    use std::io::BufReader;

    // use super::SystemUpdateFn;

    // #[test]
    // fn test() {
    //     let file = std::fs::File::open("data/dataset.sbml").expect("cannot open file");
    //     let br = std::io::BufReader::new(file);

    //     let reader = xml::reader::EventReader::new(br);
    //     let mut reader = crate::LoudReader::new(reader); // uncomment to see how xml is loaded

    //     crate::find_start_of(&mut reader, "listOfTransitions").expect("cannot find start of list");
    //     let system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
    //         super::SystemUpdateFn::try_from_xml(&mut reader).unwrap();

    //     let mut valuation = system_update_fn.get_default_partial_valuation();
    //     let some_domain = system_update_fn
    //         .named_symbolic_domains
    //         .get("todo some existing name")
    //         .unwrap();

    //     println!("line 184");
    //     some_domain.encode_bits(&mut valuation, &1);
    // }

    #[test]
    fn test_all_bigger() {
        std::fs::read_dir("data/large")
            .expect("could not read dir")
            .skip(1)
            .take(1)
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

                let smart_system_update_fn: SmartSystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                    SmartSystemUpdateFn::try_from_xml(&mut counting)
                        .expect("cannot load smart system update fn");

                println!(
                    "available variables: {:?}",
                    smart_system_update_fn.named_symbolic_domains
                );

                // let currently_reachable = smart_system_update_fn
                //     .update_fns
                //     .get("BCat_exp_id99")
                //     .unwrap()
                //     .

                let empty_subset = smart_system_update_fn.get_empty_state_subset();
                let whole_subset = smart_system_update_fn.get_whole_state_space_subset();

                let empty_succs = smart_system_update_fn
                    .transition_under_variable("BCat_exp_id99", &empty_subset);
                let whole_succs = smart_system_update_fn
                    .transition_under_variable("BCat_exp_id99", &whole_subset);

                println!("empty succs: {:?}", empty_succs.is_false());
                println!("whole succs: {:?}", whole_succs.is_true()); // actually this not being true might be the correct behavior

                // let _system_update_fn: SystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                //     SystemUpdateFn::try_from_xml(&mut counting)
                //         .expect("cannot load system update fn");

                // println!("file size = {:?}", counting.curr_line);
                // println!(
                //     "just the transitions list = {:?}",
                //     counting.curr_line - start
                // );

                // println!("system_update_fn: {:?}", system_update_fn);
            })
    }

    // #[test]
    // fn test_all_bigger_with_debug_xml_reader() {
    //     std::fs::read_dir("data/faulty")
    //         .expect("could not read dir")
    //         .for_each(|dirent| {
    //             println!("dirent = {:?}", dirent);
    //             let dirent = dirent.expect("could not read file");

    //             let xml = xml::reader::EventReader::new(std::io::BufReader::new(
    //                 std::fs::File::open(dirent.path()).unwrap(),
    //             ));

    //             let mut counting = crate::CountingReader::new(xml);

    //             crate::find_start_of(&mut counting, "listOfTransitions")
    //                 .expect("could not find list");

    //             let all_update_fns = super::load_all_update_fns(&mut counting)
    //                 .expect("could not even load the damn thing");

    //             let xml = xml::reader::EventReader::new(std::io::BufReader::new(
    //                 std::fs::File::open(dirent.path()).unwrap(),
    //             ));
    //             let mut debug_xml = crate::DebuggingReader::new(xml, &all_update_fns, true, true);

    //             // while let Ok(_) = debug_xml.next() {}

    //             loop {
    //                 if let xml::reader::XmlEvent::EndDocument = debug_xml.next().unwrap() {
    //                     break;
    //                 }
    //             }

    //             // let _system_update_fn: SystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
    //             //     super::SystemUpdateFn::try_from_xml(&mut counting)
    //             //         .expect("cannot load system update fn");

    //             // println!("file size = {:?}", counting.curr_line);
    //             // println!(
    //             //     "just the transitions list = {:?}",
    //             //     counting.curr_line - start
    //             // );

    //             // println!("system_update_fn: {:?}", system_update_fn);
    //         })
    // }

    // #[test]
    // fn test_on_test_data() {
    //     let mut reader = xml::reader::EventReader::new(std::io::BufReader::new(
    //         std::fs::File::open("data/update_fn_test.sbml").unwrap(),
    //     ));

    //     crate::find_start_of(&mut reader, "listOfTransitions").expect("cannot find start of list");
    //     let system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
    //         super::SystemUpdateFn::try_from_xml(&mut reader).unwrap();

    //     let mut valuation = system_update_fn.get_default_partial_valuation();
    //     let domain_renamed = system_update_fn
    //         .named_symbolic_domains
    //         .get("renamed")
    //         .unwrap();
    //     println!("line 217");
    //     domain_renamed.encode_bits(&mut valuation, &6);

    //     let res =
    //         system_update_fn.get_result_bits("renamed", &valuation.clone().try_into().unwrap());

    //     let mut new_valuation = valuation.clone();
    //     res.into_iter().for_each(|(bool, var)| {
    //         new_valuation.set_value(var, bool);
    //     });

    //     println!("valuation: {:?}", valuation);
    //     println!("new_valuation: {:?}", new_valuation);

    //     let successors = system_update_fn.get_successors(&valuation.clone().try_into().unwrap());
    //     println!("successors: {:?}", successors);
    // }
}
