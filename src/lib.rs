/// A private module which stores the implementation of the traits/structures relevant for
/// symbolic encoding of logical models.
///
/// TODO:
///     In the final library, we should re-export the relevant types from this module here.
mod symbolic_domain;

pub use symbolic_domain::{
    GenericIntegerDomain, GenericStateSpaceDomain, SymbolicDomain, UnaryIntegerDomain,
};

pub fn add(x: i32, y: i32) -> i32 {
    x + y
}

// expose the prototype module
mod prototype;
pub use prototype::*;

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    pub fn test() {
        assert_eq!(5, add(2, 3));
    }

    #[test]
    pub fn test_foo() {
        super::foo();
    }

    #[test]
    pub fn test_tutorial() {
        super::tutorial();
    }

    // #[test]
    // pub fn test_sol() {
    //     super::node_processing();
    // }

    #[test]
    pub fn test_sbml_model() {
        super::trying();
    }

    #[test]
    pub fn test_sbml_xml_rs() {
        let xml = r#"<apply>
            <lt/>
            <cn type="integer">5</cn>
            <ci>    x    </ci>
        </apply>
        <apply>
            <eq/>
            <ci>x</ci>
            <cn type="integer">6</cn>
        </apply>
        "#;
        let mut xml = xml::reader::EventReader::new(xml.as_bytes());
        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    if name.to_string() == "apply" {
                        println!("parsed apply {:?}", super::parse_apply_element(&mut xml));
                    }
                }
                Ok(xml::reader::XmlEvent::EndDocument) => {
                    println!("end of document");
                    break;
                }
                Err(err) => {
                    println!("err: {:?}", err);
                    break;
                }
                _ => {}
            }
        }

        // super::parse_apply_element(&mut xml::reader::EventReader::new(xml.as_bytes()));
    }
}
