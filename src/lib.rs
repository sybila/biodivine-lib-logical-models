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
            <ci>x</ci>
        </apply>"#;
        let mut xml = xml::reader::EventReader::new(xml.as_bytes());
        loop {
            if let Ok(xml::reader::XmlEvent::StartElement { name, .. }) = xml.next() {
                println!("got the apply name: {:?}", name);
                break;
            }
        }
        let res = super::parse_apply_element(&mut xml);
        println!(
            "res: {:?}",
            match res {
                Ok(res) => format!("{:?}", res),
                Err(err) => err.to_string(),
            }
        );

        // super::parse_apply_element(&mut xml::reader::EventReader::new(xml.as_bytes()));
    }
}
