use std::io::BufRead;
use xml::reader::EventReader;

use super::sbml_xml_rs::Proposition;

pub enum Expression {
    Terminal(Proposition),
    // Internal(Box<Expression>),
    Not(Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Xor(Box<Expression>, Box<Expression>),
    Implies(Box<Expression>, Box<Expression>),
}

impl Expression {
    pub fn dflt() -> Self {
        unimplemented!("default expression")
    }

    // todo consider iterative approach instead of recursive?
    pub fn try_from_xml<T: BufRead>(
        xml: &mut EventReader<T>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    match name.local_name.as_str() {
                        "not" => unimplemented!(),
                        "and" => unimplemented!(),
                        "or" => unimplemented!(),
                        "xor" => unimplemented!(),
                        "implies" => unimplemented!(),
                        must_be_cmp_op => {
                            // todo oh fck already consumed the operator; need to pass it to parse_apply_element
                            let proposition = super::parse_apply_element(xml)?; // pass the op somehow
                            drain(xml, "apply")?; // clean the xml iterator; will be used
                            return Ok(Expression::Terminal(proposition));
                        }
                    }
                }
                Ok(xml::reader::XmlEvent::EndElement { name, .. }) => {
                    if name.local_name == "apply" {
                        return Ok(Expression::dflt()); // todo not default ofc; build
                    }
                }
                Ok(xml::reader::XmlEvent::EndDocument) => {
                    return Err("unexpected end of document".into())
                }
                Err(e) => panic!("Error: {}", e),
                _ => (),
            }
        }
    }
}

/// consume the resut of the xml iterator, until the appropriate closing tag is found
fn drain<T: BufRead>(
    xml: &mut EventReader<T>,
    stop: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match xml.next() {
            Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                return Err(format!("unexpected start element: {}", name.local_name).into());
            }
            Ok(xml::reader::XmlEvent::EndElement { name, .. }) => {
                if name.local_name == stop {
                    return Ok(());
                }
            }
            Ok(xml::reader::XmlEvent::EndDocument) => {
                return Err("unexpected end of document".into());
            }
            Err(e) => {
                return Err(e.into());
            }
            _ => (),
        }
    }
}
