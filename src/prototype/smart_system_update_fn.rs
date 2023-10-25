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

    /// converts the given bdd into a dot string with names relevant to this system
    pub fn bdd_to_dot_string(&self, bdd: &Bdd) -> String {
        bdd.to_dot_string(&self.bdd_variable_set, false)
    }

    // todo this should likely not be part of the api; there are the primed variables - implementation detail
    pub fn get_bdd_variable_set(&self) -> BddVariableSet {
        self.bdd_variable_set.0.clone()
    }

    pub fn get_bdd_for_each_value_of_each_variable(&self) -> Vec<Bdd> {
        self.named_symbolic_domains
            .iter()
            .flat_map(|(var_name, sym_dom)| {
                let all_possible_values = sym_dom.get_all_possible_values(&self.bdd_variable_set.0);
                all_possible_values.into_iter().map(|possible_value| {
                    let bits = sym_dom.encode_bits_into_vec(possible_value);
                    let vars = sym_dom.symbolic_variables();
                    let vars_and_bits = vars.into_iter().zip(bits);

                    let const_true = self.bdd_variable_set.0.mk_true();

                    // constrain this specific sym variable to its specific value (& leave others unrestricted)
                    vars_and_bits.fold(const_true, |acc, (var, bit)| acc.var_select(var, bit))
                })
            })
            .collect()
    }

    pub fn get_bdd_for_each_value_of_each_variable_with_debug_but_only_not_primed(
        &self,
    ) -> Vec<(String, u8, Bdd)> {
        self.named_symbolic_domains
            .iter()
            .filter(|(name, _)| !name.contains('\''))
            .flat_map(|(var_name, sym_dom)| {
                let all_possible_values = sym_dom.get_all_possible_values(&self.bdd_variable_set.0);
                all_possible_values.into_iter().map(|possible_value| {
                    let bits = sym_dom.encode_bits_into_vec(possible_value);
                    let vars = sym_dom.symbolic_variables();
                    let vars_and_bits = vars.into_iter().zip(bits);

                    let const_true = self.bdd_variable_set.0.mk_true();

                    // constrain this specific sym variable to its specific value (& leave others unrestricted)
                    let bdd =
                        vars_and_bits.fold(const_true, |acc, (var, bit)| acc.var_select(var, bit));

                    (var_name.clone(), possible_value, bdd)
                })
            })
            .collect()
    }

    pub fn get_bdd_with_specific_var_set_to_specific_value(
        &self,
        variable_name: &str,
        value: u8,
    ) -> Bdd {
        let sym_dom = self.named_symbolic_domains.get(variable_name).unwrap();
        let bits = sym_dom.encode_bits_into_vec(value);
        let vars = sym_dom.symbolic_variables();

        let vars_and_bits = vars.into_iter().zip(bits);

        let const_true = self.bdd_variable_set.0.mk_true();

        // constrain this specific sym variable to its specific value (& leave others unrestricted)
        vars_and_bits.fold(const_true, |acc, (var, bit)| acc.var_select(var, bit))
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
    use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
    use xml::EventReader;

    use crate::{
        prototype::smart_system_update_fn::{self, SmartSystemUpdateFn},
        symbolic_domain::{BinaryIntegerDomain, GrayCodeIntegerDomain, PetriNetIntegerDomain},
        SymbolicDomain, SystemUpdateFn, UnaryIntegerDomain, XmlReader,
    };

    // use std:io::{BufRead, BufReader}

    use std::{
        cell::RefCell,
        io::BufReader,
        ops::Add,
        sync::{Arc, RwLock},
    };
    use std::{collections::HashMap, io::BufRead};

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

                let (
                    smart_empty_succs_dot,
                    smart_whole_succs_dot,
                    smart_empty_succs,
                    smart_whole_succs,
                ) = {
                    let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                        std::fs::File::open(dirent.path()).unwrap(),
                    ));

                    crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

                    let smart_system_update_fn: SmartSystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                        SmartSystemUpdateFn::try_from_xml(&mut xml)
                            .expect("cannot load smart system update fn");

                    println!(
                        "smart const false: {}",
                        smart_system_update_fn.get_empty_state_subset()
                    );
                    println!(
                        "smart const true: {}",
                        smart_system_update_fn.get_whole_state_space_subset()
                    );

                    let empty_subset = smart_system_update_fn.get_empty_state_subset();
                    let whole_subset = smart_system_update_fn.get_whole_state_space_subset();

                    let empty_succs = smart_system_update_fn
                        .transition_under_variable("BCat_exp_id99", &empty_subset);
                    let whole_succs = smart_system_update_fn
                        .transition_under_variable("BCat_exp_id99", &whole_subset);

                    (
                        smart_system_update_fn.bdd_to_dot_string(&empty_succs),
                        smart_system_update_fn.bdd_to_dot_string(&whole_succs),
                        empty_succs,
                        whole_succs,
                    )
                };

                let (
                    force_empty_succs_dot,
                    force_whole_succs_dot,
                    force_empty_succs,
                    force_whole_succs,
                ) = {
                    let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                        std::fs::File::open(dirent.path()).unwrap(),
                    ));

                    crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

                    let force_system_update_fn: SystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                        SystemUpdateFn::try_from_xml(&mut xml)
                            .expect("cannot load smart system update fn");

                    println!(
                        "force const false: {}",
                        force_system_update_fn.get_empty_state_subset()
                    );
                    println!(
                        "force const true: {}",
                        force_system_update_fn.get_whole_state_space_subset()
                    );

                    let empty_subset = force_system_update_fn.get_empty_state_subset();
                    let whole_subset = force_system_update_fn.get_whole_state_space_subset();

                    let empty_succs = force_system_update_fn
                        .transition_under_variable("BCat_exp_id99", &empty_subset);
                    let whole_succs = force_system_update_fn
                        .transition_under_variable("BCat_exp_id99", &whole_subset);

                    (
                        force_system_update_fn.bdd_to_dot_string(&empty_succs),
                        force_system_update_fn.bdd_to_dot_string(&whole_succs),
                        empty_succs,
                        whole_succs,
                    )
                };

                // assert_eq!(smart_empty_succs, force_empty_succs);
                // assert_eq!(smart_whole_succs, force_whole_succs);

                assert_eq!(smart_empty_succs_dot, force_empty_succs_dot);

                // println!("smart_empty_succs_dot = {:?}", smart_empty_succs_dot);
                // println!("force_empty_succs_dot = {:?}", force_empty_succs_dot);

                print!("smart_whole_succs_dot = {}", smart_whole_succs_dot);
                print!("force_whole_succs_dot = {}", force_whole_succs_dot);

                assert_eq!(smart_whole_succs_dot, force_whole_succs_dot); // todo this is the problematic one

                // assert!(smart_empty_succs.iff(&force_empty_succs).is_true());
                // assert!(smart_whole_succs.iff(&force_whole_succs).is_true());

                println!("smart_empty_succs = {:?}", smart_empty_succs);
                println!("smart_whole_succs = {:?}", smart_whole_succs);
                println!("force_empty_succs = {:?}", force_empty_succs);
                println!("force_whole_succs = {:?}", force_whole_succs);
            })
    }

    #[test]
    fn test_handmade_basic() {
        let (
            smart_empty_succs_dot,
            smart_whole_succs_dot,
            smart_empty_succs_bdd,
            smart_whole_succs_bdd,
        ) = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open("data/manual/basic_transition.sbml")
                    .expect("cannot open the file"),
            ));

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let smart_system_update_fn: SmartSystemUpdateFn<UnaryIntegerDomain, u8> =
                SmartSystemUpdateFn::try_from_xml(&mut xml)
                    .expect("cannot load smart system update fn");

            println!(
                "smart const false: {}",
                smart_system_update_fn.get_empty_state_subset()
            );
            println!(
                "smart const true: {}",
                smart_system_update_fn.get_whole_state_space_subset()
            );

            let empty_subset = smart_system_update_fn.get_empty_state_subset();
            let whole_subset = smart_system_update_fn.get_whole_state_space_subset();

            let empty_succs = smart_system_update_fn
                .transition_under_variable("the_only_variable", &empty_subset);
            let whole_succs = smart_system_update_fn
                .transition_under_variable("the_only_variable", &whole_subset);

            let whole_succs =
                smart_system_update_fn.transition_under_variable("the_only_variable", &whole_succs);

            (
                smart_system_update_fn.bdd_to_dot_string(&empty_succs),
                smart_system_update_fn.bdd_to_dot_string(&whole_succs),
                empty_succs,
                whole_succs,
            )
        };

        let (
            force_empty_succs_dot,
            force_whole_succs_dot,
            force_empty_succs_bdd,
            force_whole_succs_bdd,
        ) = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open("data/manual/basic_transition.sbml")
                    .expect("cannot open the file"),
            ));

            println!("---force");

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let force_system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
                SystemUpdateFn::try_from_xml(&mut xml).expect("cannot load smart system update fn");

            let empty_subset = force_system_update_fn.get_empty_state_subset();
            let whole_subset = force_system_update_fn.get_whole_state_space_subset();

            let empty_succs = force_system_update_fn
                .transition_under_variable("the_only_variable", &empty_subset);
            let whole_succs = force_system_update_fn
                .transition_under_variable("the_only_variable", &whole_subset);

            (
                force_system_update_fn.bdd_to_dot_string(&empty_succs),
                force_system_update_fn.bdd_to_dot_string(&whole_succs),
                empty_succs,
                whole_succs,
            )
        };

        // write the dot strings to files

        println!(
            "smart whole is true: {}, is false: {}",
            smart_whole_succs_bdd.is_true(),
            smart_whole_succs_bdd.is_false()
        );
        println!(
            "force whole is true: {}, is false: {}",
            force_whole_succs_bdd.is_true(),
            force_whole_succs_bdd.is_false()
        );

        assert_eq!(smart_whole_succs_dot, force_whole_succs_dot);

        assert_eq!(smart_empty_succs_dot, force_empty_succs_dot);

        // let the_two_whole = format!("{}\n{}", smart_whole_succs_dot, force_whole_succs_dot);
        let the_two_empty = format!("{}\n{}", smart_empty_succs_dot, force_empty_succs_dot);

        // std::fs::write("dot_output.dot", the_two_whole).expect("cannot write to file");
        std::fs::write("dot_output.dot", the_two_empty).expect("cannot write to file");
    }

    #[test]
    fn test_handmade_larger_starting_set() {
        let filepath = "data/manual/single_variable_v2.sbml";

        let smart_system_update_fn = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open(filepath.clone()).expect("cannot open the file"),
            ));

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let smart_system_update_fn: SmartSystemUpdateFn<UnaryIntegerDomain, u8> =
                SmartSystemUpdateFn::try_from_xml(&mut xml)
                    .expect("cannot load smart system update fn");

            smart_system_update_fn
        };

        let force_system_update_fn = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open(filepath).expect("cannot open the file"),
            ));

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let force_system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
                SystemUpdateFn::try_from_xml(&mut xml).expect("cannot load smart system update fn");

            force_system_update_fn
        };

        let smart_zero_bdd = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 0);
        let smart_two_bdd = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 2);
        let smart_zero_or_two_bdd = smart_zero_bdd.or(&smart_two_bdd);

        let force_zero_bdd = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 0);
        let force_two_bdd = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 2);
        let force_zero_or_two_bdd = force_zero_bdd.or(&force_two_bdd);

        let smart_transitioned = smart_system_update_fn
            .transition_under_variable("the_only_variable", &smart_zero_or_two_bdd);

        let force_transitioned = force_system_update_fn
            .transition_under_variable("the_only_variable", &force_zero_or_two_bdd);

        // let the_two_transitioned = format!(
        //     "{}\n{}",
        //     smart_system_update_fn.bdd_to_dot_string(&smart_transitioned),
        //     force_system_update_fn.bdd_to_dot_string(&force_transitioned)
        // );

        // std::fs::write("dot_output.dot", the_two_transitioned).expect("cannot write to file");

        assert_eq!(
            smart_system_update_fn.bdd_to_dot_string(&smart_transitioned),
            force_system_update_fn.bdd_to_dot_string(&force_transitioned)
        );

        let force_one = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 1);
        let force_three = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 3);
        let force_one_or_three = force_one.or(&force_three);

        assert!(
            force_one_or_three.iff(&force_transitioned).is_true(),
            "should be set of [one, three]",
        );
    }

    #[test]
    fn test_handmade_larger_starting_set_and_includes_previous() {
        let filepath = "data/manual/single_variable_v3.sbml";

        let smart_system_update_fn = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open(filepath.clone()).expect("cannot open the file"),
            ));

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let smart_system_update_fn: SmartSystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                SmartSystemUpdateFn::try_from_xml(&mut xml)
                    .expect("cannot load smart system update fn");

            smart_system_update_fn
        };

        let force_system_update_fn = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open(filepath).expect("cannot open the file"),
            ));

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let force_system_update_fn: SystemUpdateFn<BinaryIntegerDomain<u8>, u8> =
                SystemUpdateFn::try_from_xml(&mut xml).expect("cannot load smart system update fn");

            force_system_update_fn
        };

        let smart_zero_bdd = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 0);
        let smart_one_bdd = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 1);
        let smart_zero_or_one_bdd = smart_zero_bdd.or(&smart_one_bdd);

        let force_zero_bdd = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 0);
        let force_one_bdd = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 1);
        let force_zero_or_one_bdd = force_zero_bdd.or(&force_one_bdd);

        let smart_transitioned = smart_system_update_fn
            .transition_under_variable("the_only_variable", &smart_zero_or_one_bdd);

        let force_transitioned = force_system_update_fn
            .transition_under_variable("the_only_variable", &force_zero_or_one_bdd);

        let the_two_transitioned = format!(
            "{}\n{}",
            smart_system_update_fn.bdd_to_dot_string(&smart_transitioned),
            force_system_update_fn.bdd_to_dot_string(&force_transitioned)
        );

        std::fs::write("dot_output.dot", the_two_transitioned).expect("cannot write to file");

        assert_eq!(
            smart_system_update_fn.bdd_to_dot_string(&smart_transitioned),
            force_system_update_fn.bdd_to_dot_string(&force_transitioned)
        );

        let force_one = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 1);
        let force_two = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 2);
        let force_one_or_two = force_one.or(&force_two);

        std::fs::write(
            "dot_output.dot",
            force_system_update_fn.bdd_to_dot_string(&force_one_or_two),
        )
        .expect("cannot write to file");

        // std::fs::write(
        //     "dot_output.dot",
        //     force_system_update_fn.bdd_to_dot_string(&force_transitioned),
        // )
        // .expect("cannot write to file");

        assert!(
            // force_transitioned.iff(&force_one_or_two).is_true(),
            force_system_update_fn.bdd_to_dot_string(&force_transitioned)
                == force_system_update_fn.bdd_to_dot_string(&force_one_or_two),
            "should be set of [one, two]",
        );
    }

    #[test]
    fn test_handmade_basic_() {
        std::fs::read_dir("data/large")
            .expect("could not read dir")
            .skip(1)
            .take(1)
            .for_each(|dirent| {
                println!("dirent = {:?}", dirent);
                let filepath = dirent.expect("could not read file").path();

                let smart_system_update_fn = {
                    let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                        std::fs::File::open(filepath.clone()).expect("cannot open the file"),
                    ));

                    crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

                    let smart_system_update_fn: SmartSystemUpdateFn<UnaryIntegerDomain, u8> =
                        SmartSystemUpdateFn::try_from_xml(&mut xml)
                            .expect("cannot load smart system update fn");

                    smart_system_update_fn
                };

                let force_system_update_fn = {
                    let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                        std::fs::File::open(filepath).expect("cannot open the file"),
                    ));

                    crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

                    let force_system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
                        SystemUpdateFn::try_from_xml(&mut xml)
                            .expect("cannot load smart system update fn");

                    force_system_update_fn
                };

                let smart_triple = smart_system_update_fn
                    .get_bdd_for_each_value_of_each_variable_with_debug_but_only_not_primed();

                let force_triple =
                    force_system_update_fn.get_bdd_for_each_value_of_each_variable_with_debug();

                // the orderings might be fcked up -> pair the corresponding bdds of the two
                let smart_triple_hash_map = smart_triple
                    .into_iter()
                    .map(|(name, value, bdd)| (format!("{}{}", name, value), bdd))
                    .collect::<HashMap<_, _>>();

                let force_triple_hash_map = force_triple
                    .into_iter()
                    .map(|(name, value, bdd)| (format!("{}{}", name, value), bdd))
                    .collect::<HashMap<_, _>>();

                let smart_bdd_force_bdd_tuples = smart_triple_hash_map
                    .into_iter()
                    .map(|(name_and_value, smart_bdd)| {
                        println!("name_and_value = {}", name_and_value);
                        (
                            name_and_value.clone(),
                            smart_bdd,
                            force_triple_hash_map
                                .get(&name_and_value)
                                .expect("no such bdd")
                                .clone(),
                        )
                    })
                    .collect::<Vec<_>>();

                smart_bdd_force_bdd_tuples.iter().for_each(
                    |(variable_and_value, smart_bdd, force_bdd)| {
                        println!("comparing bdds of {}", variable_and_value);
                        assert_eq!(
                            smart_system_update_fn.bdd_to_dot_string(smart_bdd),
                            force_system_update_fn.bdd_to_dot_string(force_bdd)
                        );
                    },
                );

                let var_names = force_system_update_fn
                    .named_symbolic_domains
                    .keys()
                    .collect::<Vec<_>>();

                println!("var_names = {:?}", var_names.len());

                let those_that_eq = RwLock::new(0);
                let those_that_neq = RwLock::new(0);

                // let res =
                var_names.iter().for_each(|var_name| {
                    smart_bdd_force_bdd_tuples.iter().for_each(
                        |(name, smart_set_of_states, force_set_of_states)| {
                            println!("comparing bdds of {}", name);
                            let smart_transitioned = smart_system_update_fn
                                .transition_under_variable(var_name, smart_set_of_states);

                            let force_transitioned = force_system_update_fn
                                .transition_under_variable(var_name, force_set_of_states);

                            let smart_dot =
                                smart_system_update_fn.bdd_to_dot_string(&smart_transitioned);

                            let force_dot =
                                force_system_update_fn.bdd_to_dot_string(&force_transitioned);

                            // let the_two_whole = format!("{}\n{}", smart_whole_succs_dot, force_whole_succs_dot);
                            let the_two = format!("{}\n{}", smart_dot, force_dot);

                            // std::fs::write("dot_output.dot", the_two_whole).expect("cannot write to file");
                            std::fs::write("dot_output.dot", the_two)
                                .expect("cannot write to file");

                            assert_eq!(smart_dot, force_dot);
                            // if smart_dot != force_dot {
                            //     println!("neq");
                            // };

                            if smart_dot == force_dot {
                                let curr = {
                                    let xd = those_that_eq.read().unwrap().to_owned();
                                    xd
                                };
                                *those_that_eq.write().unwrap() = curr + 1;
                            } else {
                                let curr = {
                                    let xd = those_that_neq.read().unwrap().to_owned();
                                    xd
                                };
                                *those_that_neq.write().unwrap() = curr + 1;
                            }
                        },
                    )
                });
                // .count();

                println!("those_that_eq = {:?}", *those_that_eq.read().unwrap());
                println!("those_that_neq = {:?}", *those_that_neq.read().unwrap());

                assert_eq!(
                    *those_that_neq.read().unwrap(),
                    0,
                    "some bdds are not equal"
                );

                // println!("{:?}", res);
            });
    }

    #[test]
    fn test_handmade_basic_xd() {
        let filepath = "data/manual/handbook_example.sbml";

        let smart_system_update_fn = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open(filepath.clone()).expect("cannot open the file"),
            ));

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let smart_system_update_fn: SmartSystemUpdateFn<UnaryIntegerDomain, u8> =
                SmartSystemUpdateFn::try_from_xml(&mut xml)
                    .expect("cannot load smart system update fn");

            smart_system_update_fn
        };

        let force_system_update_fn = {
            let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
                std::fs::File::open(filepath).expect("cannot open the file"),
            ));

            crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find list");

            let force_system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
                SystemUpdateFn::try_from_xml(&mut xml).expect("cannot load smart system update fn");

            force_system_update_fn
        };

        let smart_triple = smart_system_update_fn
            .get_bdd_for_each_value_of_each_variable_with_debug_but_only_not_primed();

        let force_triple =
            force_system_update_fn.get_bdd_for_each_value_of_each_variable_with_debug();

        // the orderings might be fcked up -> pair the corresponding bdds of the two
        let smart_triple_hash_map = smart_triple
            .into_iter()
            .map(|(name, value, bdd)| (format!("{}{}", name, value), bdd))
            .collect::<HashMap<_, _>>();

        let force_triple_hash_map = force_triple
            .into_iter()
            .map(|(name, value, bdd)| (format!("{}{}", name, value), bdd))
            .collect::<HashMap<_, _>>();

        let smart_bdd_force_bdd_tuples = smart_triple_hash_map
            .into_iter()
            .map(|(name_and_value, smart_bdd)| {
                println!("name_and_value = {}", name_and_value);
                (
                    name_and_value.clone(),
                    smart_bdd,
                    force_triple_hash_map
                        .get(&name_and_value)
                        .expect("no such bdd")
                        .clone(),
                )
            })
            .collect::<Vec<_>>();

        smart_bdd_force_bdd_tuples
            .iter()
            .for_each(|(variable_and_value, smart_bdd, force_bdd)| {
                println!("comparing bdds of {}", variable_and_value);
                assert_eq!(
                    smart_system_update_fn.bdd_to_dot_string(smart_bdd),
                    force_system_update_fn.bdd_to_dot_string(force_bdd)
                );
            });

        let var_names = force_system_update_fn
            .named_symbolic_domains
            .keys()
            .collect::<Vec<_>>();

        println!("var_names = {:?}", var_names.len());

        let those_that_eq = RwLock::new(0);
        let those_that_neq = RwLock::new(0);

        // let res =
        var_names.par_iter().for_each(|var_name| {
            smart_bdd_force_bdd_tuples.par_iter().for_each(
                |(name, smart_set_of_states, force_set_of_states)| {
                    println!("comparing bdds of {}", name);
                    let smart_transitioned = smart_system_update_fn
                        .transition_under_variable(var_name, smart_set_of_states);

                    let force_transitioned = force_system_update_fn
                        .transition_under_variable(var_name, force_set_of_states);

                    let smart_dot = smart_system_update_fn.bdd_to_dot_string(&smart_transitioned);

                    let force_dot = force_system_update_fn.bdd_to_dot_string(&force_transitioned);

                    // let the_two_whole = format!("{}\n{}", smart_whole_succs_dot, force_whole_succs_dot);
                    let the_two = format!("{}\n{}", smart_dot, force_dot);

                    // std::fs::write("dot_output.dot", the_two_whole).expect("cannot write to file");
                    std::fs::write("dot_output.dot", the_two).expect("cannot write to file");

                    // assert_eq!(smart_dot, force_dot);
                    // if smart_dot != force_dot {
                    //     println!("neq");
                    // };

                    if smart_dot == force_dot {
                        let curr = {
                            let xd = those_that_eq.read().unwrap().to_owned();
                            xd
                        };
                        *those_that_eq.write().unwrap() = curr + 1;
                    } else {
                        let curr = {
                            let xd = those_that_neq.read().unwrap().to_owned();
                            xd
                        };
                        *those_that_neq.write().unwrap() = curr + 1;
                    }
                },
            )
        });
        // .count();

        println!("those_that_eq = {:?}", *those_that_eq.read().unwrap());
        println!("those_that_neq = {:?}", *those_that_neq.read().unwrap());

        assert_eq!(
            *those_that_neq.read().unwrap(),
            0,
            "some bdds are not equal"
        );

        // println!("{:?}", res);
    }
}
