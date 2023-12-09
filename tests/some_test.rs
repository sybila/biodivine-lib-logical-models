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
}

#[test]
fn some_test() {
    let the_four = TheFourImpls::<
        bio::symbolic_domain::UnaryIntegerDomain,
        bio::old_symbolic_domain::UnaryIntegerDomain,
    >::from_path("data/manual/basic_transition.sbml");

    let xd = the_four
        .old_dumb
        .get_bdd_for_each_value_of_each_variable_with_debug()[0]
        .0
        .clone();

    println!("{}", xd);

    println!("some test")
}
