#![allow(dead_code)]

// todo how to work with the "variables" that are not mentioned in the listOfTransitions?

use std::collections::HashSet;
use std::{collections::HashMap, fmt::Debug, io::BufRead};

use biodivine_lib_bdd::{
    Bdd, BddPartialValuation, BddVariable, BddVariableSet, BddVariableSetBuilder,
};
use debug_ignore::DebugIgnore;

use crate::{
    SymbolicDomain, SymbolicTransitionFn, UpdateFn, UpdateFnBdd, VariableUpdateFnCompiled,
    XmlReader,
};

#[derive(Debug)]
pub struct SmartSystemUpdateFn<D: SymbolicDomain<T>, T> {
    pub update_fns: HashMap<String, SymbolicTransitionFn<D, T>>,
    pub named_symbolic_domains: HashMap<String, D>,
    bdd_variable_set: DebugIgnore<BddVariableSet>,
}

impl<D: SymbolicDomain<u8> + Debug> SmartSystemUpdateFn<D, u8> {
    /// expects the xml reader to be at the start of the <listOfTransitions> element
    pub fn try_from_xml<XR: XmlReader<BR>, BR: BufRead>(
        xml: &mut XR,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let var_names_and_upd_fns = load_all_update_fns(xml)?;
        let sorted_ctx = {
            let mut to_be_sorted = vars_and_their_max_values(&var_names_and_upd_fns)
                .into_iter()
                .collect::<Vec<_>>();
            to_be_sorted.sort_unstable_by_key(|it| it.0.to_owned());
            to_be_sorted
        };

        // todo currently, we have no way of adding those variables, that do not have their VariableUpdateFn
        // todo  (ie their qual:transition in the xml) into the named_symbolic_domains, even tho they migh
        // todo  be used as inputs to some functions, causing panic
        let mut bdd_variable_set_builder = BddVariableSetBuilder::new();

        let named_symbolic_domains = sorted_ctx
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

        let mut unit_set = variable_set.mk_true();
        for var in var_names_and_upd_fns.keys() {
            let domain = named_symbolic_domains.get(var).unwrap();
            unit_set = unit_set.and(&domain.unit_collection(&variable_set));
            // let primed_var = format!("{}'", var);
            // let primed_domain = named_symbolic_domains.get(&primed_var).unwrap();
            // unit_set = unit_set.and(&primed_domain.unit_collection(&variable_set));
        }

        // todo this should not be necessary but you never know; actually maybe the fact that we were doing `into_values` might have fcked stuff up
        let sorted_var_names_and_upd_fns = {
            let mut to_be_sorted = var_names_and_upd_fns.into_iter().collect::<Vec<_>>();
            to_be_sorted.sort_unstable_by_key(|it| it.0.to_owned());
            to_be_sorted
        };

        let update_fns = sorted_var_names_and_upd_fns
            .into_iter()
            .map(|(_, update_fn)| {
                (
                    update_fn.target_var_name.clone(), // todo should have been ok to iterate unsorted; bcz target_var_name present here
                    UpdateFnBdd::from_update_fn(update_fn, &variable_set, &named_symbolic_domains)
                        .into(),
                )
            })
            .collect::<HashMap<String, VariableUpdateFnCompiled<D, u8>>>();

        let sorted_update_fns = {
            let mut to_be_sorted = update_fns.into_iter().collect::<Vec<_>>();
            to_be_sorted.sort_unstable_by_key(|it| it.0.to_owned());
            to_be_sorted
        };

        let smart_update_fns = sorted_update_fns
            .into_iter()
            // .map(|(target_variable_name, compiled_update_fn)| {
            .map(|tuple| {
                let target_variable_name = tuple.0;
                let compiled_update_fn = tuple.1;
                let mut pair = (
                    target_variable_name.clone(),
                    SymbolicTransitionFn::from_update_fn_compiled(
                        &compiled_update_fn,
                        &variable_set,
                        &target_variable_name,
                    ),
                );
                // TODO:
                //   Here, we normalize the transition relation to only include valid states.
                //   In theory, this normalization could be also performed at some other location
                //   (e.g. in `from_update_fn_compiled`). It also does not need the whole
                //   `unit_set`: it only requires the unit sets of the symbolic domains that are
                //   relevant for this update function. But AFAIK, this seems to be the most
                //   "convenient" place to do it unless we refactor a lot of stuff.
                pair.1.transition_function = pair.1.transition_function.and(&unit_set);
                pair
            })
            .collect::<HashMap<_, _>>();

        // this seems to always be ok = is the same every time
        // let bdd_variables = variable_set.variables();
        // let bdd_vars_to_their_names = variable_set
        //     .variables()
        //     .into_iter()
        //     .map(|var| variable_set.name_of(var))
        //     .collect::<String>();

        // let expected = "p_v1p'_v1q_v1q'_v1";

        // if bdd_vars_to_their_names != expected {
        //     panic!(
        //         "bdd_vars_to_their_names = {:?}, expected = {:?}",
        //         bdd_vars_to_their_names, expected
        //     );
        // } else {
        //     panic!("ok");
        // }

        // this seems fine too

        // let sorted_names = {
        //     let mut to_be_sorted = smart_update_fns.keys().collect::<Vec<_>>();
        //     to_be_sorted.sort_unstable();
        //     to_be_sorted
        // };

        // let all_bdds = sorted_names
        //     .into_iter()
        //     .map(|name| {
        //         let update_fn = smart_update_fns.get(name).unwrap().to_owned().to_owned();
        //         let bdd_str = update_fn
        //             .transition_function
        //             .to_dot_string(&variable_set, false);
        //         bdd_str
        //     })
        //     .collect::<String>();

        // let expected = r#"digraph G {
        //     init__ [label="", style=invis, height=0, width=0];
        //     init__ -> 2;
        //     0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
        //     1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
        //     2[label="p'_v1"];
        //     2 -> 0 [style=filled];
        //     2 -> 1 [style=dotted];
        //     }
        //     digraph G {
        //     init__ [label="", style=invis, height=0, width=0];
        //     init__ -> 4;
        //     0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
        //     1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
        //     2[label="q'_v1"];
        //     2 -> 1 [style=filled];
        //     2 -> 0 [style=dotted];
        //     3[label="q'_v1"];
        //     3 -> 0 [style=filled];
        //     3 -> 1 [style=dotted];
        //     4[label="p_v1"];
        //     4 -> 2 [style=filled];
        //     4 -> 3 [style=dotted];
        //     }
        //     "#;

        // let expected = Self::unindent(expected);

        // if all_bdds != expected {
        //     println!("{:?}", all_bdds);
        //     println!("{:?}", expected);
        //     panic!("nok");
        // } else {
        //     panic!("ok");
        // }

        Ok(Self {
            update_fns: smart_update_fns,
            named_symbolic_domains,
            bdd_variable_set: variable_set.into(),
        })
    }

    fn unindent(s: &str) -> String {
        s.lines()
            .map(|line| line.trim_start())
            .collect::<Vec<&str>>()
            .join("\n")
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

        let correct_order_of_variables_to_be_renamed = {
            let mut variables_to_be_ordered = target_symbolic_domain.symbolic_variables();
            variables_to_be_ordered.sort_unstable();
            // variables_to_be_ordered.reverse(); // not here
            variables_to_be_ordered
        };

        for bdd_variable in correct_order_of_variables_to_be_renamed {
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

    pub fn predecessors_under_variable(
        &self,
        transitioned_variable_name: &str,
        current_state: &Bdd,
    ) -> Bdd {
        // todo steps:
        // prime the input state
        // get the transition function for the given variable
        // intersect the primed input state with the transition function
        // existential projection on the primed variables of the intersected state

        let used_transition_fn = self
            .update_fns
            .get(transitioned_variable_name)
            .expect("no update function for given variable name");

        let target_symbolic_domain = self
            .named_symbolic_domains
            .get(transitioned_variable_name)
            .expect("no such variable");

        let target_symbolic_domain_primed = self
            .named_symbolic_domains
            .get(&format!("{}'", transitioned_variable_name))
            .expect("no such variable");

        // Check that no primed variable is used in `current_state`.
        let primed = self.primed_variables();
        let support_set = current_state.support_set();
        assert!(primed.iter().all(|v| !support_set.contains(v)));

        let symbolic_variables_sorted = {
            let mut symbolic_variables_to_be_sorted = target_symbolic_domain.symbolic_variables();
            symbolic_variables_to_be_sorted.sort_unstable();
            symbolic_variables_to_be_sorted.reverse();
            symbolic_variables_to_be_sorted
        };

        // let current_state_primed = target_symbolic_domain
        //     .symbolic_variables()
        let current_state_primed = symbolic_variables_sorted
            .into_iter()
            // .rev() // todo this `rev` fixes it -> must order the renaming of the variables properly -> must not rely on the order returned by symbolic_variables() -> must sort
            .fold(
                current_state.clone(),
                |mut acc, bdd_variable_to_be_primed| {
                    let bdd_variable_primed = crate::prototype::utils::find_bdd_variables_prime(
                        &bdd_variable_to_be_primed,
                        target_symbolic_domain,
                        target_symbolic_domain_primed,
                    );
                    unsafe {
                        acc.rename_variable(bdd_variable_to_be_primed, bdd_variable_primed);
                    }
                    acc
                },
            );

        let states_capable_of_transitioning_into_current = used_transition_fn
            .transition_function
            .and(&current_state_primed);

        /*println!(
            "is true {}",
            states_capable_of_transitioning_into_current.is_true()
        );*/

        target_symbolic_domain_primed
            .symbolic_variables()
            .into_iter()
            .fold(
                states_capable_of_transitioning_into_current,
                |acc, primed_variable| {
                    /*println!(
                        "restricting variable with name {}",
                        self.bdd_variable_set.name_of(primed_variable)
                    );*/
                    acc.var_exists(primed_variable)
                },
            )
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
    pub fn get_bdd_variable_set(&self) -> &BddVariableSet {
        &self.bdd_variable_set.0
    }

    pub fn get_bdd_for_each_value_of_each_variable(&self) -> Vec<Bdd> {
        self.named_symbolic_domains
            .iter()
            .flat_map(|(_var_name, sym_dom)| {
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

    /// The list of system variables, sorted in ascending order (i.e. the order in which they
    /// also appear within the BDDs).
    pub fn get_system_variables(&self) -> Vec<String> {
        let mut variables = self.update_fns.keys().cloned().collect::<Vec<_>>();
        variables.sort();
        variables
    }

    /// Returns a list of [BddVariable]-s corresponding to the encoding of the "primed"
    /// system variables.
    pub fn primed_variables(&self) -> Vec<BddVariable> {
        let mut result = Vec::new();
        for (name, domain) in &self.named_symbolic_domains {
            if name.contains("\'") {
                result.append(&mut domain.symbolic_variables());
            }
        }
        result
    }

    /// Returns a list of [BddVariable]-s corresponding to the encoding of the standard
    /// (i.e. "un-primed") system variables.
    pub fn standard_variables(&self) -> Vec<BddVariable> {
        let mut result = Vec::new();
        for (name, domain) in &self.named_symbolic_domains {
            if !name.contains("\'") {
                result.append(&mut domain.symbolic_variables());
            }
        }
        result
    }

    /// Compute the [Bdd] which represents the set of all vertices admissible in this
    /// [SmartSystemUpdateFn]. Normally, this would just be the `true` BDD, but if the
    /// encoding contains some invalid values, these need to be excluded.
    ///
    /// Note that this only concerns the "standard" system variables. The resulting BDD
    /// does not depend on the "primed" system variables.
    pub fn unit_vertex_set(&self) -> Bdd {
        let mut result = self.bdd_variable_set.mk_true();
        for var in &self.get_system_variables() {
            let domain = self.named_symbolic_domains.get(var).unwrap();
            result = result.and(&domain.unit_collection(&self.bdd_variable_set));
        }
        result
    }
}

/// expects the xml reader to be at the start of the <listOfTransitions> element
fn load_all_update_fns<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    // todo generic
) -> Result<HashMap<String, UpdateFn<u8>>, Box<dyn std::error::Error>> {
    let mut function_map: HashMap<String, UpdateFn<u8>> = crate::process_list(
        "listOfTransitions",
        "transition",
        |xml, _unused_opening_tag| UpdateFn::<u8>::try_from_xml(xml),
        xml,
    )?
    // todo this might not be the smartest nor useful; the name is already in the fn
    //  but this will allow us to access the appropriate fn quickly
    .into_iter()
    .map(|upd_fn| (upd_fn.target_var_name.clone(), upd_fn))
    .collect();

    let input_names: HashSet<String> = function_map
        .values()
        .flat_map(|it| it.input_vars_names.clone())
        .collect::<HashSet<_>>();

    for name in input_names {
        if !function_map.contains_key(&name) {
            // This variable is an input. For now, we just fix all inputs to `false`.
            // TODO: We need to handle inputs properly in the future, but not today.
            let update = UpdateFn::new(Vec::new(), name.clone(), Vec::new(), 0u8);
            function_map.insert(name, update);
        }
    }

    Ok(function_map)
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
    use biodivine_lib_bdd::BddVariableSetBuilder;
    use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
    use xml::EventReader;

    use crate::{
        prototype::smart_system_update_fn::{self, SmartSystemUpdateFn},
        symbolic_domain::{BinaryIntegerDomain, GrayCodeIntegerDomain, PetriNetIntegerDomain},
        SymbolicDomain, SystemUpdateFn, UnaryIntegerDomain, XmlReader,
    };

    // use std:io::{BufRead, BufReader}

    use core::panic;
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
    fn test_handbook() {
        let filepath = "data/manual/handbook_example.sbml";

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

        let smart_zero_bdd =
            smart_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 0);
        let smart_one_bdd =
            smart_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 1);
        let smart_zero_or_one_bdd = smart_zero_bdd.or(&smart_one_bdd);

        let force_zero_bdd =
            force_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 0);
        let force_one_bdd =
            force_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 1);
        let force_zero_or_one_bdd = force_zero_bdd.or(&force_one_bdd);

        let smart_transitioned =
            smart_system_update_fn.transition_under_variable("p", &smart_zero_or_one_bdd);

        let force_transitioned =
            force_system_update_fn.transition_under_variable("p", &force_zero_or_one_bdd);

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

        // let force_one = force_system_update_fn
        //     .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 1);
        // let force_two = force_system_update_fn
        //     .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 2);
        // let force_one_or_two = force_one.or(&force_two);

        // std::fs::write(
        //     "dot_output.dot",
        //     force_system_update_fn.bdd_to_dot_string(&force_one_or_two),
        // )
        // .expect("cannot write to file");

        // // std::fs::write(
        // //     "dot_output.dot",
        // //     force_system_update_fn.bdd_to_dot_string(&force_transitioned),
        // // )
        // // .expect("cannot write to file");

        // assert!(
        //     // force_transitioned.iff(&force_one_or_two).is_true(),
        //     force_system_update_fn.bdd_to_dot_string(&force_transitioned)
        //         == force_system_update_fn.bdd_to_dot_string(&force_one_or_two),
        //     "should be set of [one, two]",
        // );
    }

    #[test]
    fn test_preds_handbook() {
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

        let smart_p_zero =
            smart_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 0);
        let smart_p_one =
            smart_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 1);
        let smart_q_zero =
            smart_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("q", 0);
        let smart_q_one =
            smart_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("q", 1);

        let force_p_zero =
            force_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 0);
        let force_p_one =
            force_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("p", 1);
        let force_q_zero =
            force_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("q", 0);
        let force_q_one =
            force_system_update_fn.get_bdd_with_specific_var_set_to_specific_value("q", 1);

        let smart_zero_p_and_q = smart_p_one.and(&smart_q_one);
        let force_zero_p_and_q = force_p_one.and(&force_q_one);

        let smart_preds =
            smart_system_update_fn.predecessors_under_variable("p", &smart_zero_p_and_q);

        let force_preds =
            force_system_update_fn.predecessors_under_variable("p", &force_zero_p_and_q);

        let the_two_transitioned = format!(
            "{}\n{}",
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        std::fs::write("dot_output.dot", the_two_transitioned).expect("cannot write to file");

        assert_eq!(
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

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
                // println!("name_and_value = {}", name_and_value);
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
                // just asserting that we have matched the initial bdds (sets of states) correctly
                assert_eq!(
                    smart_system_update_fn.bdd_to_dot_string(smart_bdd),
                    force_system_update_fn.bdd_to_dot_string(force_bdd)
                );
            });

        let var_names = force_system_update_fn
            .named_symbolic_domains
            .keys()
            .collect::<Vec<_>>();

        // println!("var_names = {:?}", var_names.len());

        let those_that_eq = RwLock::new(0);
        let those_that_neq = RwLock::new(0);

        // let res =
        var_names.par_iter().take(1).for_each(|var_name| {
            smart_bdd_force_bdd_tuples.par_iter().take(1).for_each(
                |(name, smart_set_of_states, force_set_of_states)| {
                    // println!("comparing bdds of {}", name);
                    let smart_transitioned = smart_system_update_fn
                        .predecessors_under_variable(var_name, smart_set_of_states);

                    let force_transitioned = force_system_update_fn
                        .predecessors_under_variable(var_name, force_set_of_states);

                    let smart_dot = smart_system_update_fn.bdd_to_dot_string(&smart_transitioned);

                    let force_dot = force_system_update_fn.bdd_to_dot_string(&force_transitioned);

                    // let the_two_whole = format!("{}\n{}", smart_whole_succs_dot, force_whole_succs_dot);
                    let the_two = format!("{}\n{}", smart_dot, force_dot);

                    std::fs::write("dot_output.dot", the_two).expect("cannot write to file");

                    // assert_eq!(smart_dot, force_dot);

                    if smart_dot == force_dot {
                        let mut writer = those_that_eq.write().unwrap();
                        *writer = writer.add(1);
                    } else {
                        let mut writer = those_that_neq.write().unwrap();
                        *writer = writer.add(1);
                    }

                    println!(
                        "those_that_eq = {:?}, neq = {:?}",
                        *those_that_eq.read().unwrap(),
                        *those_that_neq.read().unwrap()
                    );
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

    #[test]
    fn test_preds_loop() {
        let filepath = "data/manual/single_var_loop.sbml";

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

        let smart_0 = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 0);
        let smart_1 = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 1);
        let smart_2 = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 2);
        let smart_3 = smart_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 3);

        let force_0 = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 0);
        let force_1 = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 1);
        let force_2 = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 2);
        let force_3 = force_system_update_fn
            .get_bdd_with_specific_var_set_to_specific_value("the_only_variable", 3);

        println!("############ 0");
        let smart_preds =
            smart_system_update_fn.predecessors_under_variable("the_only_variable", &smart_0);

        let force_preds =
            force_system_update_fn.predecessors_attempt_2("the_only_variable", &force_0);

        let the_two_transitioned = format!(
            "{}\n{}",
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        std::fs::write("dot_output.dot", the_two_transitioned).expect("cannot write to file");

        assert_eq!(
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        // 1
        println!("############ 1");
        let smart_preds =
            smart_system_update_fn.predecessors_under_variable("the_only_variable", &smart_1);

        let force_preds =
            force_system_update_fn.predecessors_attempt_2("the_only_variable", &force_1);

        let the_two_transitioned = format!(
            "{}\n{}",
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        std::fs::write("dot_output.dot", the_two_transitioned).expect("cannot write to file");

        assert_eq!(
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        // 2
        println!("############ 2");
        let smart_preds =
            smart_system_update_fn.predecessors_under_variable("the_only_variable", &smart_2);

        let force_preds =
            force_system_update_fn.predecessors_attempt_2("the_only_variable", &force_2);

        let the_two_transitioned = format!(
            "{}\n{}",
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        std::fs::write("dot_output.dot", the_two_transitioned).expect("cannot write to file");

        assert_eq!(
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        // 3
        println!("############ 3");
        let smart_preds =
            smart_system_update_fn.predecessors_under_variable("the_only_variable", &smart_3);

        let force_preds =
            force_system_update_fn.predecessors_attempt_2("the_only_variable", &force_3);

        let the_two_transitioned = format!(
            "{}\n{}",
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
        );

        std::fs::write("dot_output.dot", the_two_transitioned).expect("cannot write to file");

        assert_eq!(
            smart_system_update_fn.bdd_to_dot_string(&smart_preds),
            force_system_update_fn.bdd_to_dot_string(&force_preds)
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
    fn test_kinda_inclusive() {
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
                        // println!("name_and_value = {}", name_and_value);
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
                        // just asserting that we have matched the initial bdds (sets of states) correctly
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

                // println!("var_names = {:?}", var_names.len());

                let those_that_eq = RwLock::new(0);
                let those_that_neq = RwLock::new(0);

                // let res =
                var_names.iter().for_each(|var_name| {
                    smart_bdd_force_bdd_tuples.iter().for_each(
                        |(name, smart_set_of_states, force_set_of_states)| {
                            // println!("comparing bdds of {}", name);
                            let smart_transitioned = smart_system_update_fn
                                .transition_under_variable(var_name, smart_set_of_states);

                            // let smart_predecessors_of_successors = smart_system_update_fn
                            //     .predecessors_under_variable(&var_name, &smart_transitioned);

                            // assert!(smart_set_of_states
                            //     .iff(&smart_predecessors_of_successors)
                            //     .is_true());

                            let force_transitioned = force_system_update_fn
                                .transition_under_variable(var_name, force_set_of_states);

                            let smart_dot =
                                smart_system_update_fn.bdd_to_dot_string(&smart_transitioned);

                            let force_dot =
                                force_system_update_fn.bdd_to_dot_string(&force_transitioned);

                            // let the_two_whole = format!("{}\n{}", smart_whole_succs_dot, force_whole_succs_dot);
                            let the_two = format!("{}\n{}", smart_dot, force_dot);

                            // std::fs::write("dot_output.dot", the_two_whole).expect("cannot write to file");
                            // std::fs::write("dot_output.dot", the_two)
                            //     .expect("cannot write to file");

                            assert_eq!(smart_dot, force_dot);
                            // if smart_dot != force_dot {
                            //     println!("neq");
                            // };

                            if smart_dot == force_dot {
                                let mut writer = those_that_eq.write().unwrap();
                                *writer = writer.add(1);
                            } else {
                                let mut writer = those_that_neq.write().unwrap();
                                *writer = writer.add(1);
                            }

                            println!(
                                "those_that_eq = {:?}, neq = {:?}",
                                *those_that_eq.read().unwrap(),
                                *those_that_neq.read().unwrap()
                            );
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
    fn test_preds_kinda_inclusive() {
        std::fs::read_dir("data/large")
            .expect("could not read dir")
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
                        // println!("name_and_value = {}", name_and_value);
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
                        // just asserting that we have matched the initial bdds (sets of states) correctly
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

                // println!("var_names = {:?}", var_names.len());

                let those_that_eq = RwLock::new(0);
                let those_that_neq = RwLock::new(0);

                // let res =
                // let var_names = &var_names[..1];
                // var_names.par_iter().for_each(|var_name| {
                var_names.iter().for_each(|var_name| {
                    // let smart_bdd_force_bdd_tuples = &smart_bdd_force_bdd_tuples[..1];
                    // smart_bdd_force_bdd_tuples.par_iter().for_each(
                    smart_bdd_force_bdd_tuples.iter().for_each(
                        |(name, smart_set_of_states, force_set_of_states)| {
                            // println!("comparing bdds of {}", name);
                            let smart_preds = smart_system_update_fn
                                .predecessors_under_variable(var_name, smart_set_of_states);

                            let force_preds = force_system_update_fn
                                .predecessors_attempt_2(var_name, force_set_of_states);

                            let smart_dot = smart_system_update_fn.bdd_to_dot_string(&smart_preds);

                            let force_dot = force_system_update_fn.bdd_to_dot_string(&force_preds);

                            // let the_two_whole = format!("{}\n{}", smart_whole_succs_dot, force_whole_succs_dot);
                            let the_two = format!("{}\n{}", smart_dot, force_dot);

                            // std::fs::write("dot_output.dot", the_two_whole).expect("cannot write to file");
                            std::fs::write("dot_output.dot", the_two)
                                .expect("cannot write to file");

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

                            println!(
                                "those_that_eq = {:?}, neq = {:?}",
                                *those_that_eq.read().unwrap(),
                                *those_that_neq.read().unwrap()
                            );
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
    fn test_bdd_variable_set_ordering() {
        for _ in 0..10000 {
            let mut bdd_variable_set_builder = BddVariableSetBuilder::new();

            ('a'..='z').for_each(|c| {
                bdd_variable_set_builder.make_variable(&c.to_string());
            });

            let built = bdd_variable_set_builder.build();

            let all_vars = ('a'..='z')
                .map(|var| {
                    let bdd_var = built.var_by_name(var.to_string().as_str()).unwrap();
                    let string = format!("{}", bdd_var);
                    string
                })
                .collect::<String>();

            let exp = "012345678910111213141516171819202122232425";
            assert_eq!(all_vars, exp);

            let variables = format!("{:?}", built.variables());
            let exp_variables = "[BddVariable(0), BddVariable(1), BddVariable(2), BddVariable(3), BddVariable(4), BddVariable(5), BddVariable(6), BddVariable(7), BddVariable(8), BddVariable(9), BddVariable(10), BddVariable(11), BddVariable(12), BddVariable(13), BddVariable(14), BddVariable(15), BddVariable(16), BddVariable(17), BddVariable(18), BddVariable(19), BddVariable(20), BddVariable(21), BddVariable(22), BddVariable(23), BddVariable(24), BddVariable(25)]";
            assert_eq!(variables, exp_variables);

            let bdd_vars_to_their_names = built
                .variables()
                .into_iter()
                .map(|var| built.name_of(var))
                .collect::<String>();

            let expected_vars_to_their_names = "abcdefghijklmnopqrstuvwxyz";

            assert_eq!(bdd_vars_to_their_names, expected_vars_to_their_names);
        }
    }

    #[test]
    fn test_handmade_basic_most_interesting() {
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

        let smart_triple_sorted = {
            let mut smart_triple_sorted = smart_triple_hash_map
                .clone()
                .into_iter()
                .collect::<Vec<_>>();
            smart_triple_sorted
                .sort_unstable_by_key(|(name_and_value, _smart_bdd)| name_and_value.to_string());
            smart_triple_sorted
        };

        let force_triple_sorted = {
            let mut force_triple_sorted = force_triple_hash_map
                .clone()
                .into_iter()
                .collect::<Vec<_>>();
            force_triple_sorted
                .sort_unstable_by_key(|(name_and_value, _smart_bdd)| name_and_value.to_string());
            force_triple_sorted
        };

        let smart_triple_sorted_string_expected =
            "p0|4,0,0|4,1,1|0,1,0|p1|4,0,0|4,1,1|0,0,1|q0|4,0,0|4,1,1|2,1,0|q1|4,0,0|4,1,1|2,0,1|";

        let smart_triple_sorted_string = smart_triple_sorted
            .iter()
            .map(|(name_and_value, bdd)| format!("{}{}", name_and_value, bdd))
            .collect::<String>();

        let force_triple_sorted_string = force_triple_sorted
            .iter()
            .map(|(name_and_value, bdd)| format!("{}{}", name_and_value, bdd))
            .collect::<String>();

        println!(
            "smart_triple_sorted_string = {}",
            smart_triple_sorted_string
        );
        println!(
            "force_triple_sorted_string = {}",
            force_triple_sorted_string
        );

        assert_eq!(
            smart_triple_sorted_string,
            smart_triple_sorted_string_expected
        );

        // panic!("okk");

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

        smart_triple_sorted
            .iter()
            .zip(force_triple_sorted.clone())
            .for_each(|(smart_bdd, force_bdd)| {
                assert_eq!(
                    smart_system_update_fn.bdd_to_dot_string(&smart_bdd.1),
                    force_system_update_fn.bdd_to_dot_string(&force_bdd.1)
                );
            });

        smart_bdd_force_bdd_tuples
            .iter()
            .for_each(|(variable_and_value, smart_bdd, force_bdd)| {
                println!("comparing bdds of {}", variable_and_value);
                assert_eq!(
                    smart_system_update_fn.bdd_to_dot_string(smart_bdd),
                    force_system_update_fn.bdd_to_dot_string(force_bdd)
                );
            });

        //todo

        let var_names = force_system_update_fn
            .named_symbolic_domains
            .keys()
            .collect::<Vec<_>>();

        println!("var_names = {:?}", var_names.len());

        let those_that_eq = RwLock::new(0);
        let those_that_neq = RwLock::new(0);

        let sorted_smart_and_force_bdd_tuples = smart_triple_sorted
            .iter()
            .cloned()
            .zip(force_triple_sorted.iter().cloned())
            .map(|((_, smart_bdd), (_, force_bdd))| (smart_bdd, force_bdd))
            .collect::<Vec<_>>();

        let tuples_string = sorted_smart_and_force_bdd_tuples
            .iter()
            .map(|(smart_bdd, force_bdd)| {
                format!(
                    "{}\n{}",
                    smart_system_update_fn.bdd_to_dot_string(smart_bdd),
                    force_system_update_fn.bdd_to_dot_string(force_bdd)
                )
            })
            .collect::<String>();

        println!("tuples_string = {}\n#####", tuples_string);

        let expected_tuples_string = r#"digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="p_v1"];
            2 -> 0 [style=filled];
            2 -> 1 [style=dotted];
            }
            
            digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="p_v1"];
            2 -> 0 [style=filled];
            2 -> 1 [style=dotted];
            }
            digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="p_v1"];
            2 -> 1 [style=filled];
            2 -> 0 [style=dotted];
            }
            
            digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="p_v1"];
            2 -> 1 [style=filled];
            2 -> 0 [style=dotted];
            }
            digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="q_v1"];
            2 -> 0 [style=filled];
            2 -> 1 [style=dotted];
            }
            
            digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="q_v1"];
            2 -> 0 [style=filled];
            2 -> 1 [style=dotted];
            }
            digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="q_v1"];
            2 -> 1 [style=filled];
            2 -> 0 [style=dotted];
            }
            
            digraph G {
            init__ [label="", style=invis, height=0, width=0];
            init__ -> 2;
            0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
            1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
            2[label="q_v1"];
            2 -> 1 [style=filled];
            2 -> 0 [style=dotted];
            }
            "#;

        let expected_tuples_string = unindent(expected_tuples_string);

        assert_eq!(tuples_string, expected_tuples_string);

        // panic!("okk");

        let var_names_sorted = {
            let mut var_names_sorted = var_names.clone();
            var_names_sorted.sort_unstable();
            var_names_sorted
        };

        var_names_sorted.iter().take(1).for_each(|var_name| {
            sorted_smart_and_force_bdd_tuples.iter().take(1).for_each(
                |(smart_set_of_states, force_set_of_states)| {
                    let force_transitioned = force_system_update_fn
                        .transition_under_variable(var_name, force_set_of_states);
                    let smart_transitioned = smart_system_update_fn
                        .transition_under_variable(var_name, smart_set_of_states);

                    let formatted = format!(
                        "{}\n{}",
                        smart_system_update_fn.bdd_to_dot_string(&smart_set_of_states),
                        force_system_update_fn.bdd_to_dot_string(&force_set_of_states)
                    );

                    let expected_formatted = r#"digraph G {
                        init__ [label="", style=invis, height=0, width=0];
                        init__ -> 2;
                        0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
                        1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
                        2[label="p_v1"];
                        2 -> 0 [style=filled];
                        2 -> 1 [style=dotted];
                        }
                        
                        digraph G {
                        init__ [label="", style=invis, height=0, width=0];
                        init__ -> 2;
                        0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
                        1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
                        2[label="p_v1"];
                        2 -> 0 [style=filled];
                        2 -> 1 [style=dotted];
                        }
                        "#;

                    let expected_formatted = unindent(expected_formatted);

                    // assert_eq!(formatted, expected_formatted, "expected formatted");

                    let smart_dot = smart_system_update_fn.bdd_to_dot_string(&smart_transitioned);
                    let force_dot = force_system_update_fn.bdd_to_dot_string(&force_transitioned);

                    let the_two = format!("{}\n{}", smart_dot, force_dot);

                    std::fs::write("dot_output.dot", the_two).expect("cannot write to file");

                    assert_eq!(smart_dot, force_dot, "expected dot");

                    // todo bruh what the fuck - any of the following asserts fail/pass nondeterministically -> transition_under_variable nondeterministic
                    // println!("smart_dot = ~~~{}~~~`", smart_dot);

                    // let expected_smart_dot = r#"digraph G {
                    //     init__ [label="", style=invis, height=0, width=0];
                    //     init__ -> 3;
                    //     0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
                    //     1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
                    //     2[label="q_v1"];
                    //     2 -> 0 [style=filled];
                    //     2 -> 1 [style=dotted];
                    //     3[label="p_v1"];
                    //     3 -> 0 [style=filled];
                    //     3 -> 2 [style=dotted];
                    //     }
                    //     "#;

                    // let expected_smart_dot = unindent(expected_smart_dot);

                    // assert_eq!(smart_dot, expected_smart_dot, "expected smart dot");

                    // println!("force_dot = ~~~{}~~~`", force_dot);

                    // let expected_force_dot = r#"digraph G {
                    //     init__ [label="", style=invis, height=0, width=0];
                    //     init__ -> 3;
                    //     0 [shape=box, label="0", style=filled, shape=box, height=0.3, width=0.3];
                    //     1 [shape=box, label="1", style=filled, shape=box, height=0.3, width=0.3];
                    //     2[label="p_v1"];
                    //     2 -> 0 [style=filled];
                    //     2 -> 1 [style=dotted];
                    //     3[label="q_v1"];
                    //     3 -> 0 [style=filled];
                    //     3 -> 2 [style=dotted];
                    //     }
                    //     "#;

                    // let expected_force_dot = unindent(expected_force_dot);

                    // assert_eq!(force_dot, expected_force_dot, "expected force dot");

                    // let smart_bdd = format!("{}", smart_transitioned);

                    // println!("smart_bdd = ~~~{}~~~", smart_bdd);

                    // let expected_smart_bdd = r#"|4,0,0|4,1,1|0,1,0|"#;

                    // assert_eq!(smart_bdd, expected_smart_bdd, "expected smart bdd");

                    // let force_bdd = format!("{}", force_transitioned);

                    // println!("force_bdd = ~~~{}~~~", force_bdd);

                    // let expected_force_bdd = r#"|2,0,0|2,1,1|1,1,0|"#;

                    // assert_eq!(force_bdd, expected_force_bdd, "expected force bdd");
                },
            )
        });

        // // let res =
        // var_names.iter().for_each(|var_name| {
        //     sorted_smart_and_force_bdd_tuples.iter().for_each(
        //         |(smart_set_of_states, force_set_of_states)| {
        //             let smart_transitioned = smart_system_update_fn
        //                 .transition_under_variable(var_name, smart_set_of_states);

        //             let force_transitioned = force_system_update_fn
        //                 .transition_under_variable(var_name, force_set_of_states);

        //             let smart_dot = smart_system_update_fn.bdd_to_dot_string(&smart_transitioned);

        //             let force_dot = force_system_update_fn.bdd_to_dot_string(&force_transitioned);

        //             // let the_two_whole = format!("{}\n{}", smart_whole_succs_dot, force_whole_succs_dot);
        //             let the_two = format!("{}\n{}", smart_dot, force_dot);

        //             // std::fs::write("dot_output.dot", the_two_whole).expect("cannot write to file");
        //             std::fs::write("dot_output.dot", the_two).expect("cannot write to file");

        //             // assert_eq!(smart_dot, force_dot);
        //             // if smart_dot != force_dot {
        //             //     println!("neq");
        //             // };

        //             if smart_dot == force_dot {
        //                 let curr = {
        //                     let xd = those_that_eq.read().unwrap().to_owned();
        //                     xd
        //                 };
        //                 *those_that_eq.write().unwrap() = curr + 1;
        //             } else {
        //                 let curr = {
        //                     let xd = those_that_neq.read().unwrap().to_owned();
        //                     xd
        //                 };
        //                 *those_that_neq.write().unwrap() = curr + 1;
        //             }
        //         },
        //     )
        // });
        // // .count();

        // println!("those_that_eq = {:?}", *those_that_eq.read().unwrap());
        // println!("those_that_neq = {:?}", *those_that_neq.read().unwrap());

        // assert_eq!(
        //     *those_that_neq.read().unwrap(),
        //     0,
        //     "some bdds are not equal"
        // );

        // println!("{:?}", res);
    }

    #[test]
    fn test_demonstrate_nondeterminism() {
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
        let smart_triple_sorted = {
            let mut smart_triple_sorted = smart_triple
                .clone()
                .into_iter()
                .map(|(name, val, bdd)| (format!("{}-{}", name, val), bdd))
                .collect::<Vec<_>>();
            smart_triple_sorted
                .sort_unstable_by_key(|(name_and_value, _smart_bdd)| name_and_value.to_string());
            smart_triple_sorted
        };

        let force_triple_sorted = {
            let mut force_triple_sorted = force_triple
                .clone()
                .into_iter()
                .map(|(name, val, bdd)| (format!("{}-{}", name, val), bdd))
                .collect::<Vec<_>>();
            force_triple_sorted
                .sort_unstable_by_key(|(name_and_value, _smart_bdd)| name_and_value.to_string());
            force_triple_sorted
        };

        // those will not pass - smart has some extra variables - the primed ones -> must compare the structure using the dot string
        // smart_triple_sorted
        //     .iter()
        //     .zip(force_triple_sorted.clone())
        //     .for_each(|(smart_bdd, force_bdd)| {
        //         assert_eq!(format!("{}", smart_bdd.1), format!("{}", force_bdd.1),);
        //     });

        smart_triple_sorted
            .iter()
            .zip(force_triple_sorted.clone())
            .for_each(|(smart_bdd, force_bdd)| {
                assert_eq!(
                    smart_system_update_fn.bdd_to_dot_string(&smart_bdd.1),
                    force_system_update_fn.bdd_to_dot_string(&force_bdd.1)
                );
            });

        let sorted_smart_and_force_bdd_tuples = smart_triple_sorted
            .iter()
            .cloned()
            .zip(force_triple_sorted.iter().cloned())
            .map(|((_, smart_bdd), (_, force_bdd))| (smart_bdd, force_bdd))
            .collect::<Vec<_>>();

        let var_names = force_system_update_fn
            .named_symbolic_domains
            .keys()
            .collect::<Vec<_>>();

        let var_names_sorted = {
            let mut var_names_sorted = var_names.clone();
            var_names_sorted.sort_unstable();
            var_names_sorted
        };

        // var_names_sorted.iter().for_each(|var_name| {
        let res = var_names_sorted
            .iter()
            .flat_map(|var_name| {
                sorted_smart_and_force_bdd_tuples
                    .iter()
                    .enumerate()
                    // .for_each(|(idx, (smart_set_of_states, force_set_of_states))| {
                    .map(|(idx, (smart_set_of_states, force_set_of_states))| {
                        let force_transitioned = force_system_update_fn
                            .transition_under_variable(var_name, force_set_of_states);
                        let smart_transitioned = smart_system_update_fn
                            .transition_under_variable(var_name, smart_set_of_states);

                        let smart_dot =
                            smart_system_update_fn.bdd_to_dot_string(&smart_transitioned);
                        let force_dot =
                            force_system_update_fn.bdd_to_dot_string(&force_transitioned);

                        // todo odd thing is that if it fails, then it fails first at `variable p and idx 2`
                        // assert_eq!(
                        //     smart_dot, force_dot,
                        //     "dots sometimes do equal, but not this time; failure at vairable {} and idx {}",
                        //     var_name, idx
                        // );

                        // use this to show the results of all the cases here
                        smart_dot == force_dot
                    })
            })
            .collect::<Vec<_>>();

        // todo interesting is that this is either all true xor `[true, true, false, false, false, false, false, false]`
        println!("res = {:?}", res);
    }

    fn unindent(s: &str) -> String {
        s.lines()
            .map(|line| line.trim_start())
            .collect::<Vec<&str>>()
            .join("\n")
    }
}
