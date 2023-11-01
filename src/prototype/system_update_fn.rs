#![allow(dead_code)]

// todo how to work with the "variables" that are not mentioned in the listOfTransitions?

use std::{collections::HashMap, io::BufRead};

use biodivine_lib_bdd::{
    Bdd, BddPartialValuation, BddValuation, BddVariable, BddVariableSet, BddVariableSetBuilder,
};
use debug_ignore::DebugIgnore;

use crate::{SymbolicDomain, UpdateFn, UpdateFnBdd, VariableUpdateFnCompiled, XmlReader};

#[derive(Debug)]
pub struct SystemUpdateFn<D: SymbolicDomain<T>, T> {
    pub update_fns: HashMap<String, VariableUpdateFnCompiled<D, T>>,
    pub named_symbolic_domains: HashMap<String, D>,
    bdd_variable_set: DebugIgnore<BddVariableSet>,
}

impl<D: SymbolicDomain<u8>> SystemUpdateFn<D, u8> {
    /// expects the xml reader to be at the start of the <listOfTransitions> element
    pub fn try_from_xml<XR: XmlReader<BR>, BR: BufRead>(
        xml: &mut XR,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let var_names_and_upd_fns = load_all_update_fns(xml)?;
        let sorted_ctx = {
            let mut to_be_sorted_ctx = vars_and_their_max_values(&var_names_and_upd_fns)
                .into_iter()
                .collect::<Vec<_>>();
            to_be_sorted_ctx.sort_unstable_by_key(|it| it.0.to_owned());
            to_be_sorted_ctx
        };

        // todo currently, we have no way of adding those variables, that do not have their VariableUpdateFn
        // todo  (ie their qual:transition in the xml) into the named_symbolic_domains, even tho they migh
        // todo  be used as inputs to some functions, causing panic
        let mut bdd_variable_set_builder = BddVariableSetBuilder::new();
        let named_symbolic_domains = sorted_ctx
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

    pub fn transition_under_variable(
        &self,
        transitioned_variable_name: &str,
        current_state: &Bdd,
    ) -> Bdd {
        // get the domain

        // for each possible value of the domain

        //      get the update fn  // todo this can be done outside of the loop

        //      get the bits that encode the value

        //      associate them ^ with the bdd variables

        //      current state does not have the same value as the one it is transitioning to ie current state && !(any_that_has_this_value_of_var)

        //      current_state - (those of the current state that already have the variable set to the value we are transitioning to)

        //      `and` the accumulator with those that transition to the target bit

        // or them all together (base of the acc is const false)

        let domain = self
            .named_symbolic_domains
            .get(transitioned_variable_name.clone())
            .unwrap_or_else(|| panic!("could not find variable {}", transitioned_variable_name));

        // just weird but comfy way to create a constant false to start the fold
        let const_false = current_state.and(&current_state.not());
        let var_upd_fn = self // todo here
            .update_fns
            .get(transitioned_variable_name.clone())
            .unwrap();

        // seems legit

        // println!("const false {:?}", const_false);

        // let xd = domain
        //     .get_all_possible_values(&self.bdd_variable_set)
        //     .into_iter()
        //     .map(|val| format!("{:?}", val))
        //     .collect::<Vec<_>>()
        //     .join(", ");

        // println!("{}", xd);

        domain
            .get_all_possible_values(&self.bdd_variable_set)
            .into_iter()
            .fold(const_false.clone(), |acc, possible_var_val| {
                let bits_of_the_encoded_value = domain.encode_bits_into_vec(possible_var_val);

                // println!("{:?}", bits_of_the_encoded_value);

                // println!("possible value {}", possible_var_val);

                // todo this seems to be the problem; the result of this is nondeterministic
                let sym_vars = domain.symbolic_variables();
                // println!("{:?}", sym_vars);

                // todo yes this is the problem; the order of the variables is different
                let vars = self.bdd_variable_set.0.variables();
                // println!("{:?}", vars);
                let named_variables = vars
                    .iter()
                    .map(|var| (self.bdd_variable_set.name_of(var.to_owned()), var))
                    .collect::<Vec<_>>();
                // println!("named variables {:?}", named_variables);

                let vars_and_their_bits = domain
                    .symbolic_variables()
                    .into_iter()
                    .zip(bits_of_the_encoded_value.clone())
                    .collect::<Vec<_>>();

                // println!("{:?}", vars_and_their_bits);

                let const_true = current_state.or(&current_state.not());

                // this ok
                // println!("{:?}", const_true);

                let any_where_var_has_target_value = vars_and_their_bits
                    .iter()
                    .fold(const_true, |acc, (var, bit)| {
                        acc.var_select(var.to_owned(), bit.to_owned())
                    });

                // let any_where_var_has_target_value = const_true.select(&vars_and_their_bits);

                // println!(
                //     "any_where_var_has_target_value {:?}",
                //     any_where_var_has_target_value
                // );

                // panic!("ok");

                let vars_and_their_updating_bdds = var_upd_fn // todo here
                    .bit_answering_bdds
                    .iter()
                    .map(|(bdd, variable)| (variable, bdd))
                    .collect::<HashMap<_, _>>();

                // must order the bit-answering bdds the way the variables are ordered in the domain
                let correctly_ordered_bit_answered_bdds = vars_and_their_bits
                    .iter()
                    .map(|(bdd_variable, _)| {
                        vars_and_their_updating_bdds.get(bdd_variable).unwrap() // todo here
                    })
                    .collect::<Vec<_>>();

                let const_true = current_state.or(&current_state.not());

                let states_capable_of_transitioning_into_given_value =
                    correctly_ordered_bit_answered_bdds
                        .iter()
                        .zip(bits_of_the_encoded_value.iter().cloned())
                        .fold(
                            const_true,
                            |acc, (ith_bit_answering_bdd, ith_expected_bit)| {
                                if ith_expected_bit {
                                    acc.and(ith_bit_answering_bdd)
                                } else {
                                    acc.and(&ith_bit_answering_bdd.not())
                                }
                            },
                        );

                let states_from_current_state_set_capable_of_transitioning_into_given_value =
                    current_state.and(&states_capable_of_transitioning_into_given_value);

                // this restriction should "perform the transition"
                let states_forgot_the_value_of_target_sym_var =
                    states_from_current_state_set_capable_of_transitioning_into_given_value
                        // .restrict(&vars_and_their_bits[..]);
                        .exists(
                            &vars_and_their_bits
                                .iter()
                                .cloned()
                                .map(|(var, _bit)| var)
                                .collect::<Vec<_>>()[..],
                        );

                let states_transitioned_into_given_value =
                    any_where_var_has_target_value.and(&states_forgot_the_value_of_target_sym_var);

                // let res = acc.or(&states_transitioned_into_given_value);
                // todo yes those start to differ
                // println!("{:?}", res);
                // res

                acc.or(&states_transitioned_into_given_value)
            })
    }

    pub fn predecessors_under_variable(
        &self,
        transitioned_variable_name: &str,
        current_state: &Bdd,
    ) -> Bdd {
        // predecessors must have all the variables other than the target one set to the same values as `current_state`
        // -> for `current_state`, find all states that are the same, but any value of target variable (as long as it encodes valid symbolic value)

        // todo remove all the redundant cloning of transitioned_variable_name in the above funciton

        let var_upd_fn = self // todo here
            .update_fns
            .get(transitioned_variable_name.clone())
            .unwrap();

        let vars_and_their_updating_bdds = var_upd_fn // todo here
            .bit_answering_bdds
            .iter()
            .map(|(bdd, variable)| (variable, bdd))
            .collect::<HashMap<_, _>>();

        // let vars_and_their_bits = domain
        //     .symbolic_variables()
        //     .into_iter()
        //     .zip(bits_of_the_encoded_value.clone())
        //     .collect::<Vec<_>>();

        // // must order the bit-answering bdds the way the variables are ordered in the domain
        // let correctly_ordered_bit_answered_bdds = vars_and_their_bits
        //     .iter()
        //     .map(|(bdd_variable, _)| {
        //         vars_and_their_updating_bdds.get(bdd_variable).unwrap() // todo here
        //     })
        //     .collect::<Vec<_>>();

        let target_var_sym_dom = self
            .named_symbolic_domains
            .get(transitioned_variable_name)
            .unwrap_or_else(|| panic!("could not find variable {}", transitioned_variable_name));

        let bit_repr_of_all_the_possible_values_of_target_var = target_var_sym_dom
            .get_all_possible_values(&self.bdd_variable_set)
            .into_iter()
            .map(|possible_value| target_var_sym_dom.encode_bits_into_vec(possible_value));

        let const_false = current_state.and(&current_state.not());

        // sets from `current_state`, but with any value of target variable // todo swap `current_state` with `current_state_with_specific` value
        // let all_possible_states = bit_repr_of_all_the_possible_values_of_target_var.fold(
        //     const_false,
        //     |acc, bits_of_possible_value| {
        //         let current_state_with_specific_value_of_target_var = target_var_sym_dom
        //             .symbolic_variables()
        //             .into_iter()
        //             .zip(bits_of_possible_value)
        //             .fold(current_state.clone(), |acc, (bdd_var, bit_val)| {
        //                 acc.var_select(bdd_var, bit_val)
        //             });

        //         acc.or(&current_state_with_specific_value_of_target_var)
        //     },
        // );

        // let bit_repr_of_all_the_possible_values_of_target_var_and_current_states_with_target_var_fixed_to_that =
        let current_state_split_by_value_of_target_var =
            bit_repr_of_all_the_possible_values_of_target_var
                .clone()
                .map(|bits_of_possible_value| {
                    (
                        bits_of_possible_value.clone(),
                        target_var_sym_dom
                            .symbolic_variables()
                            .into_iter()
                            .zip(bits_of_possible_value)
                            .fold(current_state.clone(), |acc, (bdd_var, bit_val)| {
                                acc.var_select(bdd_var, bit_val)
                            }),
                    )
                });

        let states_with_known_val_of_target_var_and_their_predecessors =
            current_state_split_by_value_of_target_var.map(
                |(fixed_val_bits, set_of_states_with_var_with_specific_bits)| {
                    // get all possible predecessors of `set_of_states_with_var_with_specific_bits`
                    let relaxed_value_of_target_var =
                        bit_repr_of_all_the_possible_values_of_target_var
                            .clone()
                            .fold(const_false.clone(), |acc, bits_of_possible_value| {
                                let current_state_with_specific_value_of_target_var =
                                    target_var_sym_dom
                                        .symbolic_variables()
                                        .into_iter()
                                        .zip(bits_of_possible_value)
                                        .fold(
                                            set_of_states_with_var_with_specific_bits.clone(),
                                            // current_state.clone(),  // incorrect; must be specifically predecessors of that set with fixed value of target var
                                            |acc, (bdd_var, bit_val)| {
                                                acc.var_select(bdd_var, bit_val)
                                            },
                                        );

                                acc.or(&current_state_with_specific_value_of_target_var)
                            });

                    let vars_and_their_bits = target_var_sym_dom
                        .clone()
                        .symbolic_variables()
                        .into_iter()
                        .zip(fixed_val_bits.clone())
                        .collect::<Vec<_>>();

                    // must order the bit-answering bdds the way the variables are ordered in the domain
                    let correctly_ordered_bit_answered_bdds = vars_and_their_bits
                        .iter()
                        .map(|(bdd_variable, _)| {
                            vars_and_their_updating_bdds.get(bdd_variable).unwrap()
                            // todo here
                        })
                        .collect::<Vec<_>>();

                    // todo abstract the function `set_of_those_states_that_transition_to_specific_value_of_specific_variable`
                    // let those_transitioning_to_target =
                    // target_var_sym_dom
                    //     .symbolic_variables()
                    //     .into_iter()
                    //     .zip(fixed_val_bits)
                    correctly_ordered_bit_answered_bdds
                        .into_iter()
                        .zip(fixed_val_bits)
                        .fold(
                            relaxed_value_of_target_var,
                            |acc, (bit_updating_bdd, bit_val)| {
                                // let name_of_this_var = self.bdd_variable_set.name_of(bdd_var);
                                // let bdd_updating_bdd_var = {
                                //     let aux = self.update_fns.get(&name_of_this_var);
                                //     if aux.is_none() {
                                //         panic!(
                                //             "could not find variable `{}`; only available: `{}`",
                                //             name_of_this_var,
                                //             // self.bdd_variable_set
                                //             //     .0
                                //             //     .variables()
                                //             //     .iter()
                                //             //     .map(|var| self
                                //             //         .bdd_variable_set
                                //             //         .name_of(var.to_owned()))
                                //             //     .collect::<Vec<_>>()
                                //             //     .join(", ")
                                //             self.update_fns
                                //                 .keys()
                                //                 .cloned()
                                //                 .collect::<Vec<_>>()
                                //                 .join(", ")
                                //         );
                                //     }
                                //     aux.unwrap()
                                // };
                                // // todo this should already be zipped with the iterator -> would be O(n) instead of O(n^2)
                                // let bdd_updating_specific_bit = &bdd_updating_bdd_var
                                //     .bit_answering_bdds
                                //     .clone()
                                //     .into_iter()
                                //     .find_map(|(bdd, maybe_target_bdd_var)| {
                                //         if bdd_var == maybe_target_bdd_var {
                                //             Some(bdd)
                                //         } else {
                                //             None
                                //         }
                                //     })
                                //     .unwrap();

                                if bit_val {
                                    acc.and(bit_updating_bdd)
                                } else {
                                    acc.and(&bit_updating_bdd.not())
                                }
                            },
                        )
                },
            );

        states_with_known_val_of_target_var_and_their_predecessors
            .fold(const_false.clone(), |acc, to_or| acc.or(&to_or))
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

    pub fn get_bdd_for_each_value_of_each_variable_with_debug(&self) -> Vec<(String, u8, Bdd)> {
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

        // println!("line 184");
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

        // println!("system_update_fn: {:?}", system_update_fn);

        let mut valuation = system_update_fn.get_default_partial_valuation();
        let domain = system_update_fn.named_symbolic_domains.get("ORI").unwrap();
        domain.encode_bits(&mut valuation, &1);

        let succs = system_update_fn.get_successors(&valuation.try_into().unwrap());
        // println!("succs: {:?}", succs);
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
