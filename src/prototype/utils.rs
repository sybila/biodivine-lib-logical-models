use std::{io::BufRead, str::FromStr};
use xml::{
    attribute::OwnedAttribute,
    name::OwnedName,
    namespace::Namespace,
    reader::{EventReader, XmlEvent},
};

use crate::UpdateFn;

pub fn expect_opening<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
) -> Result<StartElementWrapper, Box<dyn std::error::Error>> {
    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* whitespace is the reason we want to loop */ }
            Ok(XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            }) => return Ok(StartElementWrapper::new(name, attributes, namespace)), // til abt variable binding
            other => return Err(format!("expected an opening, got {:?}", other).into()),
        }
    }
}

pub fn expect_opening_of<XR: XmlReader<BR>, BR: BufRead>(
    expected: &str,
    xml: &mut XR,
) -> Result<StartElementWrapper, Box<dyn std::error::Error>> {
    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* whitespace is the reason we want to loop */ }
            Ok(XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            }) => {
                return if name.local_name == expected {
                    Ok(StartElementWrapper::new(name, attributes, namespace))
                } else {
                    Err(format!(
                        "expected opening element {}, got {}",
                        expected, name.local_name
                    )
                    .into())
                }
            }
            other => {
                return Err(format!("expected opening of {}, got {:?}", expected, other).into())
            }
        }
    }
}

/// since XmlEvent::StartElement obviously cannot be as return type, this is used instead in cases
/// where only this version of the enum can be returned
pub struct StartElementWrapper {
    pub name: OwnedName,
    pub attributes: Vec<OwnedAttribute>,
    pub namespace: Namespace,
}

impl StartElementWrapper {
    pub fn new(name: OwnedName, attributes: Vec<OwnedAttribute>, namespace: Namespace) -> Self {
        Self {
            name,
            attributes,
            namespace,
        }
    }
}

/// todo maybe add return value as the whole end tag; so far no usecase
pub fn expect_closure_of<XR: XmlReader<BR>, BR: BufRead>(
    expected: &str,
    xml: &mut XR,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* whitespace is the reason we want to loop */ }
            Ok(XmlEvent::EndElement { name, .. }) => {
                return if name.local_name == expected {
                    Ok(())
                } else {
                    Err(format!("expected closing of {}, got {}", expected, name.local_name).into())
                }
            }
            any => return Err(format!("expected closing of {}, got {:?}", expected, any).into()),
        }
    }
}

// lmao nice type signature
/// takes care of processing xml lists into vector of given items. list_name is expected to be the
/// name of the tag wrapping the whole list. item_name is expected to be the name of the tag
/// wrapping each element. each time `item_name` is encountered, the `xml` is handed off to the
/// `processing_fn` function. if any of the calls to `processing_fn` fail, that error is returned
/// immediately (// todo append some extra info abt the fact it was from `process_list`?).
/// `processing_fn` is expected to return with the `xml` pointing to the last element of the item
/// (ie to `</ item_name>`). if any other element in the list other than `item_name` is
/// encountered, error is returned. once closing tag with `list_name` is encountered, Vec
/// containing all the processed items is returned (items in the correct order ofc)
/// since some functions for processing of items require access to the opening event of the item,
/// that shall be provided as the second argument to the `processing_fn`
pub fn process_list<XR: XmlReader<BR>, BR: BufRead, Fun, Res>(
    list_name: &str,
    item_name: &str,
    processing_fn: Fun,
    xml: &mut XR,
) -> Result<Vec<Res>, Box<dyn std::error::Error>>
where
    Fun: Fn(&mut XR, StartElementWrapper) -> Result<Res, Box<dyn std::error::Error>>,
{
    let mut acc = Vec::<Res>::new();

    loop {
        let elem = xml.next();

        match elem {
            Ok(XmlEvent::Whitespace(_)) => { /* ignore */ }
            Ok(XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            }) => {
                if name.local_name == item_name {
                    acc.push(processing_fn(
                        xml,
                        StartElementWrapper::new(name, attributes, namespace),
                    )?);
                    continue;
                }

                return Err(format!(
                    "expected opening of item {}, got {}",
                    item_name, name.local_name
                )
                .into());
            }
            Ok(XmlEvent::EndElement { name, .. }) => {
                return if name.local_name == list_name {
                    Ok(acc)
                } else {
                    Err(format!(
                        "expected closing element with name {}, got {}",
                        list_name, name.local_name
                    )
                    .into())
                }
            }
            other => {
                return Err(format!(
                    "expected either opening of {} or closing of {}, got {:?}",
                    item_name, list_name, other,
                )
                .into())
            }
        }
    }
}

/// get the update fn from "data/update_fn_test.sbml"
/// used in tests / to play around with the code
pub fn get_test_update_fn<T: FromStr>() -> UpdateFn<T> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open("data/update_fn_test.sbml").expect("cannot open file");
    let file = BufReader::new(file);

    let mut xml = xml::reader::EventReader::new(file);

    loop {
        match xml.next() {
            Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "transition" {
                    let update_fn = UpdateFn::try_from_xml(&mut xml);
                    return update_fn.unwrap();
                }
            }
            Ok(xml::reader::XmlEvent::EndElement { .. }) => continue,
            Ok(xml::reader::XmlEvent::EndDocument) => panic!(),
            Err(_) => panic!(),
            _ => continue,
        }
    }
}

/// iterates through the xml until it finds the first opening tag with the given name
/// (specifically, opening_element.name.local_name == expected_name)
pub fn find_start_of<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    expected_name: &str,
) -> Result<(), String> {
    loop {
        match xml.next() {
            Ok(xml::reader::XmlEvent::StartElement { name: n, .. })
                if n.local_name == expected_name =>
            {
                return Ok(());
            }
            Ok(xml::reader::XmlEvent::EndElement { .. }) => continue,
            Ok(xml::reader::XmlEvent::EndDocument) => return Err("end of document".to_string()),
            Err(e) => return Err(format!("error: {:?}", e)),
            _ => continue, // should be uninteresting
        }
    }
}

pub trait XmlReader<BR: BufRead> {
    fn next(&mut self) -> Result<XmlEvent, String>;
}

impl<BR: BufRead> XmlReader<BR> for EventReader<BR> {
    fn next(&mut self) -> Result<XmlEvent, String> {
        match self.next() {
            Ok(e) => Ok(e),
            Err(e) => Err(format!("error: {:?}", e)),
        }
    }
}

pub struct LoudReader<BR: BufRead> {
    xml: EventReader<BR>,
    curr_indent: usize,
}

impl<BR: BufRead> LoudReader<BR> {
    pub fn new(xml: EventReader<BR>) -> Self {
        Self {
            xml,
            curr_indent: 0,
        }
    }
}

impl<BR: BufRead> XmlReader<BR> for LoudReader<BR> {
    fn next(&mut self) -> Result<XmlEvent, String> {
        match self.xml.next() {
            Ok(e) => {
                match e.clone() {
                    XmlEvent::StartElement {
                        name,
                        // attributes,
                        // namespace,
                        ..
                    } => {
                        println!(
                            "{}<{:?}>",
                            (0..self.curr_indent).map(|_| ' ').collect::<String>(),
                            name
                        );

                        self.curr_indent += 2;
                    }
                    XmlEvent::EndElement { name, .. } => {
                        println!(
                            "{}</{:?}>",
                            (0..self.curr_indent).map(|_| ' ').collect::<String>(),
                            name
                        );

                        self.curr_indent -= 2;
                    }
                    _ => {}
                }
                // println!("xddd next: {:?}", e);
                Ok(e)
            }
            Err(e) => Err(format!("error: {:?}", e)),
        }
    }
}

pub struct CountingReader<BR: BufRead> {
    xml: EventReader<BR>,
    pub curr_line: usize,
}

impl<BR: BufRead> CountingReader<BR> {
    pub fn new(xml: EventReader<BR>) -> Self {
        Self { xml, curr_line: 0 }
    }
}

impl<BR: BufRead> XmlReader<BR> for CountingReader<BR> {
    fn next(&mut self) -> Result<XmlEvent, String> {
        match self.xml.next() {
            Ok(e) => {
                match e.clone() {
                    XmlEvent::StartElement {
                        name,
                        // attributes,
                        // namespace,
                        ..
                    } => {
                        self.curr_line += 1;
                    }
                    XmlEvent::EndElement { name, .. } => {
                        self.curr_line += 1;
                    }
                    _ => {}
                }
                // println!("xddd next: {:?}", e);
                Ok(e)
            }
            Err(e) => Err(format!("error: {:?}", e)),
        }
    }
}
