#![allow(dead_code)]

use biodivine_lib_bdd::Bdd;
use biodivine_lib_logical_models::prelude::{
    self as bio,
    symbolic_domain::{
        BinaryIntegerDomain, GrayCodeIntegerDomain, PetriNetIntegerDomain, SymbolicDomainOrd,
        UnaryIntegerDomain,
    },
};

struct TheTwoImpls<DO>
where
    DO: bio::symbolic_domain::SymbolicDomainOrd<u8>,
{
    new_dumb: bio::update_fn::SystemUpdateFn<DO, u8>,
    new_smart: bio::update_fn::SmartSystemUpdateFn<DO, u8>,
}

impl<DO> TheTwoImpls<DO>
where
    DO: bio::symbolic_domain::SymbolicDomainOrd<u8>,
{
    fn encode_one(&self, variable_name: &str, value: u8) -> TheFourImplsBdd {
        TheFourImplsBdd {
            new_dumb_bdd: self.new_dumb.encode_one(variable_name, &value),
            new_smart_bdd: self.new_smart.encode_one(variable_name, &value),
        }
    }

    fn bbd_for_each_value_of_each_variable(&self) -> Vec<TheFourImplsBdd> {
        let mut res = Vec::new();
        for (name, domain) in self.new_smart.standard_variables_names_and_domains().iter() {
            for value in domain.get_all_possible_values() {
                let new_dumb_bdd = self.new_dumb.encode_one(name, &value);
                let new_smart_bdd = self.new_smart.encode_one(name, &value);

                let the_four_impls_bdd = TheFourImplsBdd {
                    new_dumb_bdd,
                    new_smart_bdd,
                };

                res.push(the_four_impls_bdd);
            }
        }
        res
    }
}

/// useful for creating bdd for each of the four impls
/// that can be passed to `TheFourImpls`, which handles
/// the transitions
struct TheFourImplsBdd {
    new_dumb_bdd: Bdd,
    new_smart_bdd: Bdd,
}

impl TheFourImplsBdd {
    fn are_same<DO>(&self, context: &TheTwoImpls<DO>) -> bool
    where
        DO: bio::symbolic_domain::SymbolicDomainOrd<u8>,
    {
        let new_dumb_dot = context.new_dumb.bdd_to_dot_string(&self.new_dumb_bdd);
        let new_smart_dot = context.new_smart.bdd_to_dot_string(&self.new_smart_bdd);

        new_dumb_dot == new_smart_dot
    }
}

impl<DO> TheTwoImpls<DO>
where
    DO: SymbolicDomainOrd<u8>,
{
    fn from_path(sbml_path: &str) -> Self {
        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let new_dumb = bio::update_fn::SystemUpdateFn::<DO, u8>::try_from_xml(&mut xml)
            .expect("should be able to parse");

        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let new_smart = bio::update_fn::SmartSystemUpdateFn::<DO, u8>::try_from_xml(&mut xml)
            .expect("should be able to parse");

        Self {
            new_dumb,
            new_smart,
        }
    }

    fn successors_async(
        &self,
        transition_variable_name: &str,
        source_states_set: &TheFourImplsBdd,
    ) -> TheFourImplsBdd {
        let new_dumb = self
            .new_dumb
            .successors_async(transition_variable_name, &source_states_set.new_dumb_bdd);
        let new_smart = self
            .new_smart
            .successors_async(transition_variable_name, &source_states_set.new_smart_bdd);

        TheFourImplsBdd {
            new_dumb_bdd: new_dumb,
            new_smart_bdd: new_smart,
        }
    }

    fn predecessors_async(
        &self,
        transition_variable_name: &str,
        source_states_set: &TheFourImplsBdd,
    ) -> TheFourImplsBdd {
        let new_dumb = self
            .new_dumb
            .predecessors_async(transition_variable_name, &source_states_set.new_dumb_bdd);
        let new_smart = self.new_smart.predecessors_async(
            transition_variable_name,
            source_states_set.new_smart_bdd.clone(),
        );

        TheFourImplsBdd {
            new_dumb_bdd: new_dumb,
            new_smart_bdd: new_smart,
        }
    }
}

/// funciton to compare the two implementations;
/// in the future, the generics should likely be more flexible (not necessarily `u8`)
fn consistency_check<DO>()
where
    DO: SymbolicDomainOrd<u8>,
{
    std::fs::read_dir("data/large")
        .expect("could not read dir")
        .for_each(|dirent| {
            let tmp = dirent.expect("could not read dir entry").path();
            let filepath = tmp.to_str().unwrap();

            println!("dataset {}", filepath);

            let the_four = TheTwoImpls::<DO>::from_path(filepath);

            // vector of bdds, one for each value of each variable
            let simple_initial_states = the_four.bbd_for_each_value_of_each_variable();

            simple_initial_states.iter().for_each(|initial_state| {
                let variable = the_four
                    .new_smart
                    .get_system_variables()
                    .into_iter()
                    .next()
                    .expect("there should be some variable");

                assert_eq!(
                    the_four
                        .new_dumb
                        .bdd_to_dot_string(&initial_state.new_dumb_bdd),
                    the_four
                        .new_smart
                        .bdd_to_dot_string(&initial_state.new_smart_bdd),
                    "the new impls should be the same"
                );

                let transitioned = the_four.successors_async(&variable, initial_state);

                assert!(
                    transitioned.are_same(&the_four),
                    "the four impls should be the same"
                );
            });
        });
}

/// funciton to compare the two implementations;
/// in the future, the generics should likely be more flexible (not necessarily `u8`)
fn predecessors_consistency_check<DO>()
where
    DO: SymbolicDomainOrd<u8>,
{
    std::fs::read_dir("data/large")
        .expect("could not read dir")
        .for_each(|dirent| {
            let tmp = dirent.expect("could not read dir entry").path();
            let filepath = tmp.to_str().unwrap();

            println!("dataset {}", filepath);

            let the_four = TheTwoImpls::<DO>::from_path(filepath);

            let simple_initial_states = the_four.bbd_for_each_value_of_each_variable();

            for initial_state in simple_initial_states.iter() {
                let variable = the_four
                    .new_smart
                    .get_system_variables()
                    .into_iter()
                    .next()
                    .expect("there should be some variable");

                assert_eq!(
                    the_four
                        .new_dumb
                        .bdd_to_dot_string(&initial_state.new_dumb_bdd),
                    the_four
                        .new_smart
                        .bdd_to_dot_string(&initial_state.new_smart_bdd),
                    "the new impls should be the same"
                );

                assert!(initial_state.are_same(&the_four), "initial states are same");

                let transitioned = the_four.predecessors_async(&variable, initial_state);

                assert!(transitioned.are_same(&the_four), "all are same");
            }
        });
}

#[test]
fn test_consistency_successosr_unary() {
    consistency_check::<UnaryIntegerDomain>();
}

#[test]
fn test_consistency_successosr_binary() {
    consistency_check::<BinaryIntegerDomain<u8>>();
}

#[test]
fn test_consistency_successosr_petri_net() {
    consistency_check::<PetriNetIntegerDomain>();
}

#[test]
fn test_consistency_successosr_gray() {
    consistency_check::<GrayCodeIntegerDomain<u8>>();
}

#[test]
fn test_consistency_predecessors_unary() {
    predecessors_consistency_check::<UnaryIntegerDomain>();
}

#[test]
fn test_consistency_predecessors_binary() {
    predecessors_consistency_check::<BinaryIntegerDomain<u8>>();
}

#[test]
fn test_consistency_predecessors_petri_net() {
    predecessors_consistency_check::<PetriNetIntegerDomain>();
}

#[test]
fn test_consistency_predecessors_gray() {
    predecessors_consistency_check::<GrayCodeIntegerDomain<u8>>();
}
