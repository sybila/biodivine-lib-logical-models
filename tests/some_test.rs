#![allow(dead_code)]

use biodivine_lib_bdd::Bdd;
use biodivine_lib_logical_models::prelude as bio;

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

        println!("smart_are_same output: {}", old_smart_dot == new_smart_dot);

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

        // todo update
        // let new_dumb = source_states_set.new_dumb_bdd.clone();
        // let new_smart = source_states_set.new_smart_bdd.clone();l

        TheFourImplsBdd {
            old_dumb_bdd: old_dumb,
            old_smart_bdd: old_smart,
            new_dumb_bdd: new_dumb,
            new_smart_bdd: new_smart,
        }
    }

    fn are_dumb_update_fns_same(&self) -> bool {
        let old_dumb = self.old_dumb.update_fns.iter().map(|(xed, xd)| {});

        todo!()
    }
}

// #[test]
// fn some_test() {
//     let the_four = TheFourImpls::<
//         bio::symbolic_domain::UnaryIntegerDomain,
//         bio::old_symbolic_domain::UnaryIntegerDomain,
//     >::from_path("data/manual/basic_transition.sbml");

//     // let old_dumb_set = the_four.old_dumb.encode_one("the_only_variable", 1);

//     // let old_smart_set = the_four.old_smart.encode_one("the_only_variable", 1);

//     // let new_dumb_set = the_four.new_dumb.encode_one("the_only_variable", &1);

//     // let new_smart_set = the_four.new_smart.encode_one("the_only_variable", &1);

//     // //
//     // let transitioned_old_dumb = the_four
//     //     .old_dumb
//     //     .transition_under_variable("the_only_variable", &old_dumb_set);
//     // let transitioned_old_smart = the_four
//     //     .old_smart
//     //     .transition_under_variable("the_only_variable", &old_smart_set);
//     // let transitioned_new_dumb = the_four
//     //     .new_dumb
//     //     .successors_async("the_only_variable", &new_dumb_set);
//     // let transitioned_new_smart = the_four
//     //     .new_smart
//     //     .successors_async("the_only_variable", &new_smart_set);

//     // // transitioned_old_dumb.to_dot_string(variables, zero_pruned)
//     // let old_dumb_dot = the_four.old_dumb.bdd_to_dot_string(&transitioned_old_dumb);
//     // let old_smart_dot = the_four
//     //     .old_smart
//     //     .bdd_to_dot_string(&transitioned_old_smart);
//     // let new_dumb_dot = the_four.new_dumb.bdd_to_dot_string(&transitioned_new_dumb);
//     // let new_smart_dot = the_four
//     //     .new_smart
//     //     .bdd_to_dot_string(&transitioned_new_smart);

//     // assert_eq!(old_dumb_dot, old_smart_dot);
//     // assert_eq!(old_dumb_dot, new_dumb_dot);
//     // assert_eq!(old_dumb_dot, new_smart_dot);

//     // println!("old_dumb_dot {}", old_dumb_dot);
//     // println!("old_smart_do {}", old_smart_dot);
//     // println!("new_dumb_dot {}", new_dumb_dot);
//     // println!("new_smart_do {}", new_smart_dot);

//     // println!("the xd {}", xd);

//     let abstract_bdd = the_four.encode_one("the_only_variable", 1);
//     assert!(
//         abstract_bdd.are_same(&the_four),
//         "encoding the same value should result in the same bdd dot string"
//     );
//     let transitioned_abstract_bdd = the_four.successors_async("the_only_variable", &abstract_bdd);
//     assert!(
//         transitioned_abstract_bdd.are_same(&the_four),
//         "transitioning the same value should result in the same bdd dot string"
//     );

//     println!("some test")
// }

// todo facts:
// the "basic" (`data/manual`) transitions work for the new implementations
// only the larger (`data/large`) transitions fail
// this might be, because the handmade do not target cases where invalid states might come into play
//  ^ this can be seen by modifying the old impls - if pruning of invalid staes is removed in one of them,
//    the "basic" tests do not detect any difference, while the "large" tests do
// #[test]
fn consistency_check() {
    let mut i = 0usize;
    loop {
        std::fs::read_dir("data/large")
            .expect("could not read dir")
            .for_each(|dirent| {
                println!("dirent = {:?}", dirent);
                let filepath = dirent.expect("could not read file").path();

                // let filepath = "data/manual/basic_transition.sbml".to_string();
                // let filepath = "data/large/146_BUDDING-YEAST-FAURE-2009.sbml".to_string();

                let the_four = TheFourImpls::<NewDomain, OldDomain>::from_path(
                    filepath.to_str().expect("could not convert to str"),
                );
                // >::from_path(&filepath);

                // vector of bdds, one for each value of each variable
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

                    let transitioned = the_four.successors_async(variable, initial_state);

                    // todo currently, there is a discrepancy between the old and new impls
                    // todo old are unit-set-pruned -> correct
                    // assert!(
                    //     transitioned.old_are_same(&the_four),
                    //     "the old impls should be the same"
                    // );

                    // if !transitioned.new_are_same(&the_four) {
                    //     // println!("old are not the same");
                    //     println!(
                    //         "new dumb bdd = {}",
                    //         the_four
                    //             .new_dumb
                    //             .bdd_to_dot_string(&transitioned.new_dumb_bdd)
                    //     );
                    //     println!(
                    //         "new smart bdd = {}",
                    //         the_four
                    //             .new_smart
                    //             .bdd_to_dot_string(&transitioned.new_smart_bdd)
                    //     );
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

                    assert!(
                        transitioned.are_same(&the_four),
                        "the four impls should be the same"
                    );

                    println!("count = {} were the same", i);
                    i += 1;
                }
            });
    }
}

// #[test]
// fn smart_transition_debug() {
//     std::fs::read_dir("data/large") // todo large
//         .expect("could not read dir")
//         .for_each(|dirent| {
//             println!("dirent = {:?}", dirent);
//             let filepath = dirent.expect("could not read file").path();

//             // let filepath = "data/manual/basic_transition.sbml".to_string();

//             let the_four =
//                 TheFourImpls::<
//                     bio::symbolic_domain::UnaryIntegerDomain,
//                     bio::old_symbolic_domain::UnaryIntegerDomain,
//                 >::from_path(filepath.to_str().expect("could not convert to str"));
//             // >::from_path(&filepath);

//             // vector of bdds, one for each value of each variable
//             let simple_initial_states = the_four.bbd_for_each_value_of_each_variable();

//             for (count, initial_state) in simple_initial_states.iter().enumerate() {
//                 let variable = the_four
//                     .old_dumb
//                     .named_symbolic_domains
//                     .keys()
//                     .next()
//                     .expect("there should be some variable");

//                 // ensure the inputs are the same
//                 assert_eq!(
//                     the_four
//                         .new_dumb
//                         .bdd_to_dot_string(&initial_state.new_dumb_bdd),
//                     the_four
//                         .new_smart
//                         .bdd_to_dot_string(&initial_state.new_smart_bdd),
//                     "the new impls should be the same"
//                 );

//                 let transitioned = the_four.successors_async(variable, initial_state);

//                 if !transitioned.smart_are_same(&the_four) {
//                     let old_smart = the_four
//                         .old_smart
//                         .bdd_to_dot_string(&transitioned.old_smart_bdd);
//                     println!("old smart: {}", old_smart);

//                     let new_smart = the_four
//                         .new_smart
//                         .bdd_to_dot_string(&transitioned.new_smart_bdd);
//                     println!("new smart: {}", new_smart);

//                     println!("equal: {}", old_smart == new_smart);

//                     assert!(false, "the smart impls should be the same")
//                 }

//                 println!("count = {} were the same", count);
//             }
//         });
// }

// todo this is the one not working
// #[test]
fn check_specific() {
    let mut i = 0usize;
    loop {
        // let filepath = "data/manual/basic_transition.sbml".to_string();
        let filepath = "data/large/146_BUDDING-YEAST-FAURE-2009.sbml".to_string();

        let the_four = TheFourImpls::<
            NewDomain,
            OldDomain,
            // >::from_path(filepath.to_str().expect("could not convert to str"));
        >::from_path(&filepath);

        let the_four_check = TheFourImpls::<
            NewDomain,
            OldDomain,
            // >::from_path(filepath.to_str().expect("could not convert to str"));
        >::from_path(&filepath);

        // vector of bdds, one for each value of each variable
        let simple_initial_states = the_four.bbd_for_each_value_of_each_variable();

        for (count, initial_state) in simple_initial_states.iter().enumerate() {
            let variable = the_four
                .old_dumb
                .named_symbolic_domains
                .keys()
                .next()
                .expect("there should be some variable");

            // if variable != "Net1" {
            //     continue;
            // }

            // assert_eq!(
            //     the_four
            //         .new_dumb
            //         .bdd_to_dot_string(&initial_state.new_dumb_bdd),
            //     the_four
            //         .new_smart
            //         .bdd_to_dot_string(&initial_state.new_smart_bdd),
            //     "the new impls should be the same"
            // );

            let transitioned = the_four.successors_async(variable, initial_state);

            // assert!(
            //     transitioned.are_same(&the_four),
            //     "the old impls should be the same"
            // );

            i += 1;

            println!("iteration: {}", i);

            // std::thread::sleep(std::time::Duration::from_secs(10));

            // assert!(transitioned.old_are_same(&the_four));
            if !transitioned.dumb_are_same(&the_four) {
                // println!("old are not the same");
                // println!(
                //     "new dumb bdd = {}",
                //     the_four
                //         .old_dumb
                //         .bdd_to_dot_string(&transitioned.old_dumb_bdd)
                // );
                // println!(
                //     "new smart bdd = {}",
                //     the_four
                //         .new_dumb
                //         .bdd_to_dot_string(&transitioned.new_dumb_bdd)
                // );

                // println!(
                //     "transitioned under variable {} and value {}",
                //     variable,
                //     the_four
                //         .old_dumb
                //         .bdd_to_dot_string(&initial_state.old_dumb_bdd)
                // );
                // println!("transition under {} neq", variable);

                // let transition_fn_old_dumb = the_four.old_dumb.update_fns.get("Net1").unwrap();
                // let (_, (transition_fn_new_dumb, _)) = the_four
                //     .new_dumb
                //     .update_fns
                //     .iter()
                //     .find(|(name, _)| name == "Net1")
                //     .unwrap();

                // let old_bdds = transition_fn_old_dumb.bit_answering_bdds.clone();
                // let new_bdds = transition_fn_new_dumb.bit_answering_bdds.clone();

                // let old_ordered = {
                //     let mut aux = old_bdds.clone();
                //     aux.sort_by_key(|(_, bdd_var)| bdd_var.to_owned());
                //     aux
                // };
                // let new_ordered = {
                //     let mut aux = new_bdds.clone();
                //     aux.sort_by_key(|(bdd_var, _)| bdd_var.to_owned());
                //     aux
                // };

                // println!(
                //     "their lengths: old = {}, new = {}",
                //     old_ordered.len(),
                //     new_ordered.len()
                // );

                // for ((old_bdd, old_var), (new_var, new_bdd)) in
                //     old_ordered.iter().zip(new_ordered.iter())
                // {
                //     let var_eq = old_var == new_var;
                //     let bdd_eq = the_four.old_dumb.bdd_to_dot_string(old_bdd)
                //         == the_four.new_dumb.bdd_to_dot_string(new_bdd);
                //     println!("var_eq = {}, bdd_eq = {}", var_eq, bdd_eq);
                // }

                // println!("ordered properly?");
                // for (l, r) in old_bdds.iter().zip(old_ordered.iter()) {
                //     println!("l = {}, r = {}", l.1, r.1);
                // }
                // for (l, r) in new_bdds.iter().zip(new_ordered.iter()) {
                //     println!("l = {}, r = {}", l.0, r.0);
                // }

                // println!("transition bdds");
                // for ((old_bdd, _), (_, new_bdd)) in old_bdds.iter().zip(new_bdds.iter()) {
                //     let old_bdd_str = the_four.old_dumb.bdd_to_dot_string(old_bdd);
                //     let new_bdd_str = the_four.new_dumb.bdd_to_dot_string(new_bdd);
                //     println!("eq bdds; {}", old_bdd_str == new_bdd_str);
                // }

                // println!("transition bdds flipped");
                // for ((old_bdd, _), (_, new_bdd)) in old_bdds.iter().zip(new_bdds.iter().rev()) {
                //     let old_bdd_str = the_four.old_dumb.bdd_to_dot_string(old_bdd);
                //     let new_bdd_str = the_four.new_dumb.bdd_to_dot_string(new_bdd);
                //     println!("eq bdds; {}", old_bdd_str == new_bdd_str);
                // }

                // println!(
                //     "caches: \nold = {:?}, \nnew = {:?}",
                //     the_four.old_dumb.cache, the_four.new_dumb.cache
                // );

                println!("failing at {}", variable);
                println!(
                    "failing with bdd input {}",
                    the_four
                        .old_dumb
                        .bdd_to_dot_string(&initial_state.old_dumb_bdd)
                );
            }
            // assert!(transitioned.dumb_are_same(&the_four));

            // if !transitioned.smart_are_same(&the_four) {
            //     println!("under variable {}", variable);
            // }

            // assert!(
            //     transitioned.smart_are_same(&the_four),
            //     "smart are not the same"
            // );

            assert!(transitioned.are_same(&the_four));

            // assert!(transitioned.new_are_same(&the_four));

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

            // println!("count = {} were the same", count);
        }
    }
}

#[test]
fn predecessors_consistency_check() {
    let mut i = 0usize;

    loop {
        std::fs::read_dir("data/large")
            .expect("could not read dir")
            .for_each(|dirent| {
                println!("dirent = {:?}", dirent);
                let filepath = dirent.expect("could not read file").path();

                // let filepath = "data/manual/basic_transition.sbml".to_string();
                // let filepath = "data/large/146_BUDDING-YEAST-FAURE-2009.sbml".to_string();

                let the_four = TheFourImpls::<NewDomain, OldDomain>::from_path(
                    filepath.to_str().expect("could not convert to str"),
                );
                // >::from_path(&filepath);

                // vector of bdds, one for each value of each variable
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

                    assert!(initial_state.are_same(&the_four), "initial states are same");

                    let transitioned = the_four.predecessors_async(variable, initial_state);

                    // todo currently, there is a discrepancy between the old and new impls
                    // todo old are unit-set-pruned -> correct
                    // assert!(
                    //     transitioned.old_are_same(&the_four),
                    //     "the old impls should be the same"
                    // );

                    // if !transitioned.new_are_same(&the_four) {
                    //     // println!("old are not the same");
                    //     println!(
                    //         "new dumb bdd = {}",
                    //         the_four
                    //             .new_dumb
                    //             .bdd_to_dot_string(&transitioned.new_dumb_bdd)
                    //     );
                    //     println!(
                    //         "new smart bdd = {}",
                    //         the_four
                    //             .new_smart
                    //             .bdd_to_dot_string(&transitioned.new_smart_bdd)
                    //     );
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

                    // assert!(transitioned.old_are_same(&the_four), "old");

                    // assert!(transitioned.dumb_are_same(&the_four), "dumb");

                    assert!(transitioned.new_are_same(&the_four), "all are same");

                    println!("predecessors count = {} were the same", i);
                    i += 1;
                }
            });
    }
}
