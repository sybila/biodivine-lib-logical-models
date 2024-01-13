#![allow(dead_code)]

use biodivine_lib_bdd::Bdd;
use biodivine_lib_logical_models::prelude::{
    self as bio, old_symbolic_domain::SymbolicDomain, symbolic_domain::SymbolicDomain as _,
};

type OldDomain = bio::old_symbolic_domain::BinaryIntegerDomain<u8>;
type NewDomain = bio::symbolic_domain::BinaryIntegerDomain<u8>;

struct TheFourImpls<D, OD>
where
    D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
    OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
{
    old_dumb: bio::old_update_fn::SystemUpdateFn<OD, u8>,
    old_smart: bio::old_update_fn::SmartSystemUpdateFn<OD, u8>,
    new_dumb: bio::update_fn::SystemUpdateFn<D, u8>,
    new_smart: bio::update_fn::SmartSystemUpdateFn<D, u8>,
}

impl<D, OD> TheFourImpls<D, OD>
where
    D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
    OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
{
    fn encode_one(&self, variable_name: &str, value: u8) -> TheFourImplsBdd {
        TheFourImplsBdd {
            old_dumb_bdd: self.old_dumb.encode_one(variable_name, value),
            old_smart_bdd: self.old_smart.encode_one(variable_name, value),
            new_dumb_bdd: self.new_dumb.encode_one(variable_name, &value),
            new_smart_bdd: self.new_smart.encode_one(variable_name, &value),
        }
    }

    fn bbd_for_each_value_of_each_variable(&self) -> Vec<TheFourImplsBdd> {
        let mut res = Vec::new();
        for (name, domain) in self.old_dumb.named_symbolic_domains.iter() {
            for value in domain.get_all_possible_values(&self.old_dumb.bdd_variable_set) {
                // res.push(self.encode_one(name, value));

                let old_dumb_bdd = self.old_dumb.encode_one(name, value);
                let old_smart_bdd = self.old_smart.encode_one(name, value);
                let new_dumb_bdd = self.new_dumb.encode_one(name, &value);
                let new_smart_bdd = self.new_smart.encode_one(name, &value);

                let the_four_impls_bdd = TheFourImplsBdd {
                    old_dumb_bdd,
                    old_smart_bdd,
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
    old_dumb_bdd: Bdd,
    old_smart_bdd: Bdd,
    new_dumb_bdd: Bdd,
    new_smart_bdd: Bdd,
}

impl TheFourImplsBdd {
    fn are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let old_dumb_dot = context.old_dumb.bdd_to_dot_string(&self.old_dumb_bdd);
        let old_smart_dot = context.old_smart.bdd_to_dot_string(&self.old_smart_bdd);
        let new_dumb_dot = context.new_dumb.bdd_to_dot_string(&self.new_dumb_bdd);
        let new_smart_dot = context.new_smart.bdd_to_dot_string(&self.new_smart_bdd);

        old_dumb_dot == old_smart_dot
            && old_dumb_dot == new_dumb_dot
            && old_dumb_dot == new_smart_dot
    }

    // todo remove this as the `old` implementation is removed
    fn old_are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let old_dumb_dot = context.old_dumb.bdd_to_dot_string(&self.old_dumb_bdd);
        let old_smart_dot = context.old_smart.bdd_to_dot_string(&self.old_smart_bdd);

        old_dumb_dot == old_smart_dot
    }

    // todo rename; once `old` removed, there is no `new`
    fn new_are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let new_dumb_dot = context.new_dumb.bdd_to_dot_string(&self.new_dumb_bdd);
        let new_smart_dot = context.new_smart.bdd_to_dot_string(&self.new_smart_bdd);

        new_dumb_dot == new_smart_dot
    }

    // todo remove this as the `old` implementation is removed
    fn smart_are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let old_smart_dot = context.old_smart.bdd_to_dot_string(&self.old_smart_bdd);
        let new_smart_dot = context.new_smart.bdd_to_dot_string(&self.new_smart_bdd);

        old_smart_dot == new_smart_dot
    }

    // todo remove this as the `old` implementation is removed
    fn dumb_are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let old_dumb_dot = context.old_dumb.bdd_to_dot_string(&self.old_dumb_bdd);
        let new_dumb_dot = context.new_dumb.bdd_to_dot_string(&self.new_dumb_bdd);

        old_dumb_dot == new_dumb_dot
    }
}

impl TheFourImpls<NewDomain, OldDomain> {
    fn from_path(sbml_path: &str) -> Self {
        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let old_dumb = bio::old_update_fn::SystemUpdateFn::<OldDomain, u8>::try_from_xml(&mut xml)
            .expect("should be able to parse");

        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let old_smart =
            bio::old_update_fn::SmartSystemUpdateFn::<OldDomain, u8>::try_from_xml(&mut xml)
                .expect("should be able to parse");

        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let new_dumb = bio::update_fn::SystemUpdateFn::<NewDomain, u8>::try_from_xml(&mut xml)
            .expect("should be able to parse");

        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let new_smart =
            bio::update_fn::SmartSystemUpdateFn::<NewDomain, u8>::try_from_xml(&mut xml)
                .expect("should be able to parse");

        Self {
            old_dumb,
            old_smart,
            new_dumb,
            new_smart,
        }
    }

    // fn async_successors(&self)
    fn successors_async(
        &self,
        transition_variable_name: &str,
        source_states_set: &TheFourImplsBdd,
    ) -> TheFourImplsBdd {
        let old_dumb = self
            .old_dumb
            .transition_under_variable(transition_variable_name, &source_states_set.old_dumb_bdd);
        let old_smart = self
            .old_smart
            .transition_under_variable(transition_variable_name, &source_states_set.old_smart_bdd);
        let new_dumb = self
            .new_dumb
            .successors_async(transition_variable_name, &source_states_set.new_dumb_bdd);
        let new_smart = self
            .new_smart
            .successors_async(transition_variable_name, &source_states_set.new_smart_bdd);

        TheFourImplsBdd {
            old_dumb_bdd: old_dumb,
            old_smart_bdd: old_smart,
            new_dumb_bdd: new_dumb,
            new_smart_bdd: new_smart,
        }
    }

    fn predecessors_async(
        &self,
        transition_variable_name: &str,
        source_states_set: &TheFourImplsBdd,
    ) -> TheFourImplsBdd {
        let old_dumb = self
            .old_dumb
            .predecessors_attempt_2(transition_variable_name, &source_states_set.old_dumb_bdd);
        let old_smart = self.old_smart.predecessors_under_variable(
            transition_variable_name,
            &source_states_set.old_smart_bdd,
        );
        let new_dumb = self
            .new_dumb
            .predecessors_async(transition_variable_name, &source_states_set.new_dumb_bdd);
        let new_smart = self.new_smart.predecessors_async(
            transition_variable_name,
            source_states_set.new_smart_bdd.clone(),
        );

        TheFourImplsBdd {
            old_dumb_bdd: old_dumb,
            old_smart_bdd: old_smart,
            new_dumb_bdd: new_dumb,
            new_smart_bdd: new_smart,
        }
    }
}

#[test]
fn consistency_check() {
    std::fs::read_dir("data/large")
        .expect("could not read dir")
        .for_each(|dirent| {
            let tmp = dirent.expect("could not read dir entry").path();
            let filepath = tmp.to_str().unwrap();

            let the_four = TheFourImpls::<NewDomain, OldDomain>::from_path(filepath);

            // vector of bdds, one for each value of each variable
            let simple_initial_states = the_four.bbd_for_each_value_of_each_variable();

            simple_initial_states.iter().for_each(|initial_state| {
                let variable = the_four
                    .old_dumb
                    .named_symbolic_domains
                    .keys()
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

                let transitioned = the_four.successors_async(variable, initial_state);

                assert!(
                    transitioned.are_same(&the_four),
                    "the four impls should be the same"
                );
            });
        });
}

// todo factor out the functionality, provide type parameter for the method -> test for each domain implementation calling the same underlying method
// #[test]
fn predecessors_consistency_check() {
    std::fs::read_dir("data/large")
        .expect("could not read dir")
        .for_each(|dirent| {
            println!("dirent = {:?}", dirent);
            let filepath = dirent.expect("could not read file").path();

            let the_four = TheFourImpls::<NewDomain, OldDomain>::from_path(
                filepath.to_str().expect("could not convert to str"),
            );

            let simple_initial_states = the_four.bbd_for_each_value_of_each_variable();

            for initial_state in simple_initial_states.iter() {
                let variable = the_four
                    .old_dumb
                    .named_symbolic_domains
                    .keys()
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

                let old_dumb_const_true = the_four.old_dumb.bdd_variable_set.mk_true();
                let unit_set_old_dumb = the_four
                    .old_dumb
                    .update_fns
                    .iter()
                    .find(|_| true)
                    .map(|(_, update_fn)| {
                        update_fn.named_symbolic_domains.iter().fold(
                            old_dumb_const_true,
                            |acc, (_, domain)| {
                                acc.and(
                                    &domain.unit_collection(&the_four.old_dumb.bdd_variable_set),
                                )
                            },
                        )
                    })
                    .unwrap();

                let unit_set_old_dumb_str = the_four.old_dumb.bdd_to_dot_string(&unit_set_old_dumb);

                let new_dumb_const_true = the_four.new_dumb.bdd_variable_set.mk_true();
                let unit_set_new_dumb = the_four.new_dumb.update_fns.iter().fold(
                    new_dumb_const_true,
                    |acc, (_, (_, domain))| {
                        acc.and(&domain.unit_collection(&the_four.new_dumb.bdd_variable_set))
                    },
                );

                let unit_set_new_dumb_str = the_four.new_dumb.bdd_to_dot_string(&unit_set_new_dumb);

                assert!(
                    unit_set_old_dumb_str == unit_set_new_dumb_str,
                    "the unit sets should be the same"
                );

                assert!(initial_state.are_same(&the_four), "initial states are same");

                let transitioned = the_four.predecessors_async(variable, initial_state);

                if !transitioned.smart_are_same(&the_four) {
                    let old_smart_dot = the_four
                        .old_smart
                        .bdd_to_dot_string(&transitioned.old_smart_bdd);
                    println!("old smart: {}", old_smart_dot);

                    let new_smart_dot = the_four
                        .new_smart
                        .bdd_to_dot_string(&transitioned.new_smart_bdd);
                    println!("new smart: {}", new_smart_dot);
                }

                assert!(transitioned.smart_are_same(&the_four), "all are same");
            }
        });
}
