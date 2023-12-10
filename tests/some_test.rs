#![allow(dead_code)]

use biodivine_lib_bdd::Bdd;
use biodivine_lib_logical_models::prelude::{self as bio, old_symbolic_domain::UnaryIntegerDomain};

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

    fn old_are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let old_dumb_dot = context.old_dumb.bdd_to_dot_string(&self.old_dumb_bdd);
        let old_smart_dot = context.old_smart.bdd_to_dot_string(&self.old_smart_bdd);

        old_dumb_dot == old_smart_dot
    }

    fn new_are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let new_dumb_dot = context.new_dumb.bdd_to_dot_string(&self.new_dumb_bdd);
        let new_smart_dot = context.new_smart.bdd_to_dot_string(&self.new_smart_bdd);

        new_dumb_dot == new_smart_dot
    }

    fn smart_are_same<D, OD>(&self, context: &TheFourImpls<D, OD>) -> bool
    where
        D: bio::symbolic_domain::SymbolicDomainOrd<u8>,
        OD: bio::old_symbolic_domain::SymbolicDomain<u8>,
    {
        let old_smart_dot = context.old_smart.bdd_to_dot_string(&self.old_smart_bdd);
        let new_smart_dot = context.new_smart.bdd_to_dot_string(&self.new_smart_bdd);

        old_smart_dot == new_smart_dot
    }

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

impl
    TheFourImpls<
        bio::symbolic_domain::UnaryIntegerDomain,
        bio::old_symbolic_domain::UnaryIntegerDomain,
    >
{
    fn from_path(sbml_path: &str) -> Self {
        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let old_dumb =
            bio::old_update_fn::SystemUpdateFn::<UnaryIntegerDomain, u8>::try_from_xml(&mut xml)
                .expect("should be able to parse");

        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let old_smart =
            bio::old_update_fn::SmartSystemUpdateFn::<UnaryIntegerDomain, u8>::try_from_xml(
                &mut xml,
            )
            .expect("should be able to parse");

        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let new_dumb =
            bio::update_fn::SystemUpdateFn::<bio::symbolic_domain::UnaryIntegerDomain, u8>::try_from_xml(
                &mut xml,
            )
            .expect("should be able to parse");

        let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
            std::fs::File::open(sbml_path).expect("should be able to open file"),
        ));
        bio::find_start_of(&mut xml, "listOfTransitions").expect("should be able to find");
        let new_smart = bio::update_fn::SmartSystemUpdateFn::<
            bio::symbolic_domain::UnaryIntegerDomain,
            u8,
        >::try_from_xml(&mut xml)
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
}

#[test]
fn some_test() {
    let the_four = TheFourImpls::<
        bio::symbolic_domain::UnaryIntegerDomain,
        bio::old_symbolic_domain::UnaryIntegerDomain,
    >::from_path("data/manual/basic_transition.sbml");

    // let old_dumb_set = the_four.old_dumb.encode_one("the_only_variable", 1);

    // let old_smart_set = the_four.old_smart.encode_one("the_only_variable", 1);

    // let new_dumb_set = the_four.new_dumb.encode_one("the_only_variable", &1);

    // let new_smart_set = the_four.new_smart.encode_one("the_only_variable", &1);

    // //
    // let transitioned_old_dumb = the_four
    //     .old_dumb
    //     .transition_under_variable("the_only_variable", &old_dumb_set);
    // let transitioned_old_smart = the_four
    //     .old_smart
    //     .transition_under_variable("the_only_variable", &old_smart_set);
    // let transitioned_new_dumb = the_four
    //     .new_dumb
    //     .successors_async("the_only_variable", &new_dumb_set);
    // let transitioned_new_smart = the_four
    //     .new_smart
    //     .successors_async("the_only_variable", &new_smart_set);

    // // transitioned_old_dumb.to_dot_string(variables, zero_pruned)
    // let old_dumb_dot = the_four.old_dumb.bdd_to_dot_string(&transitioned_old_dumb);
    // let old_smart_dot = the_four
    //     .old_smart
    //     .bdd_to_dot_string(&transitioned_old_smart);
    // let new_dumb_dot = the_four.new_dumb.bdd_to_dot_string(&transitioned_new_dumb);
    // let new_smart_dot = the_four
    //     .new_smart
    //     .bdd_to_dot_string(&transitioned_new_smart);

    // assert_eq!(old_dumb_dot, old_smart_dot);
    // assert_eq!(old_dumb_dot, new_dumb_dot);
    // assert_eq!(old_dumb_dot, new_smart_dot);

    // println!("old_dumb_dot {}", old_dumb_dot);
    // println!("old_smart_do {}", old_smart_dot);
    // println!("new_dumb_dot {}", new_dumb_dot);
    // println!("new_smart_do {}", new_smart_dot);

    // println!("the xd {}", xd);

    let abstract_bdd = the_four.encode_one("the_only_variable", 1);
    assert!(
        abstract_bdd.are_same(&the_four),
        "encoding the same value should result in the same bdd dot string"
    );
    let transitioned_abstract_bdd = the_four.successors_async("the_only_variable", &abstract_bdd);
    assert!(
        transitioned_abstract_bdd.are_same(&the_four),
        "transitioning the same value should result in the same bdd dot string"
    );

    println!("some test")
}

// todo facts:
// the "basic" (`data/manual`) transitions work for the new implementations
// only the larger (`data/large`) transitions fail
// this might be, because the handmade do not target cases where invalid states might come into play
//  ^ this can be seen by modifying the old impls - if pruning of invalid staes is removed in one of them,
//    the "basic" tests do not detect any difference, while the "large" tests do
#[test]
fn consistency_check() {
    std::fs::read_dir("data/large") // todo large
        .expect("could not read dir")
        .for_each(|dirent| {
            println!("dirent = {:?}", dirent);
            let filepath = dirent.expect("could not read file").path();

            // let filepath = "data/manual/basic_transition.sbml".to_string();

            let the_four =
                TheFourImpls::<
                    bio::symbolic_domain::UnaryIntegerDomain,
                    bio::old_symbolic_domain::UnaryIntegerDomain,
                >::from_path(filepath.to_str().expect("could not convert to str"));
            // >::from_path(&filepath);

            // vector of bdds, one for each value of each variable
            let simple_initial_states = the_four.bbd_for_each_value_of_each_variable();

            for (count, initial_state) in simple_initial_states.into_iter().enumerate() {
                let _variable = the_four // todo use in the commented code below
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

                // let new_dumb_transitioned = the_four
                //     .new_dumb
                //     .successors_async(variable, &initial_state.new_dumb_bdd);

                // let new_smart_transitioned = the_four
                //     .new_smart
                //     .successors_async(variable, &initial_state.new_smart_bdd);

                // assert_eq!(
                //     the_four.new_dumb.bdd_to_dot_string(&new_dumb_transitioned),
                //     the_four
                //         .new_smart
                //         .bdd_to_dot_string(&new_smart_transitioned)
                // );

                // let transitioned = the_four.successors_async(variable, initial_state);

                // todo currently, there is a discrepancy between the old and new impls
                // todo old are unit-set-pruned -> correct
                // assert!(
                //     transitioned.old_are_same(&the_four),
                //     "the old impls should be the same"
                // );

                // if !transitioned.new_are_same(&the_four) {
                //     // println!("old are not the same");
                //     println!("new dumb bdd = {:?}", transitioned.old_dumb_bdd);
                //     println!("new smart bdd = {:?}", transitioned.old_smart_bdd);
                // }

                // assert!(
                //     transitioned.new_are_same(&the_four),
                //     "the new impls should be the same"
                // );

                // assert!(
                //     transitioned.smart_are_same(&the_four),
                //     "the smart impls should be the same"
                // );

                // assert!(
                //     transitioned.dumb_are_same(&the_four),
                //     "the dumb impls should be the same"
                // );

                // assert!(
                //     transitioned.are_same(&the_four),
                //     "the four impls should be the same"
                // );

                println!("count = {} were the same", count);
            }
        });
}
