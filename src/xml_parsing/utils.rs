#![allow(dead_code)]

use std::{fmt::Display, io::BufRead};
use thiserror::Error;
use xml::{attribute::OwnedAttribute, name::OwnedName, namespace::Namespace, reader::XmlEvent};

use super::xml_reader::XmlReader;

/// used for creating expected events for reporting errors
/// this way, we do not have to construct complex instances of XmlEvent
#[derive(Debug)]
pub enum ExpectedXmlEvent {
    Start(String),
    End(String),
    AnyStart,
    AnyEnd,
    Characters,
}

#[derive(Error, Debug)]
pub enum XmlReadingError {
    UnexpectedEvent {
        expected: ExpectedXmlEvent,
        got: XmlEvent,
    },
    UnderlyingReaderError(#[from] xml::reader::Error),
    ParsingError(String),
    NoSuchAttribute(String),
    WrongAmountOfElements {
        expected_amount: usize,
        found_items_string: String,
    },
}

impl Display for XmlReadingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlReadingError::UnexpectedEvent { expected, got } => write!(
                f,
                "Unexpected event. Expected: {:?}, got: {:?}",
                expected, got
            ),
            XmlReadingError::UnderlyingReaderError(e) => {
                write!(f, "Underlying reader error: {}", e)
            }
            XmlReadingError::ParsingError(s) => write!(f, "Parsing error; could not parse {}", s),
            XmlReadingError::NoSuchAttribute(s) => write!(f, "No such attribute: {}", s),
            XmlReadingError::WrongAmountOfElements {
                expected_amount,
                found_items_string,
            } => {
                write!(
                    f,
                    "Wrong amount of elements. Expected: {}, found elements: [{}]",
                    expected_amount, found_items_string
                )
            }
        }
    }
}

/// since XmlEvent::StartElement obviously cannot be as return type, this is used instead in cases
/// where only this variant of the enum can be returned
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

/// reads the next event from the xml reader and returns it if it is a start element
/// otherwise, returns an error.
/// useful when the next event should be a start element but it is not known which one
pub fn expect_opening<BR, XR>(xml: &mut XR) -> Result<StartElementWrapper, XmlReadingError>
where
    BR: BufRead,
    XR: XmlReader<BR>,
{
    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* whitespace is the reason we want to loop */ }
            Ok(XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            }) => return Ok(StartElementWrapper::new(name, attributes, namespace)),
            other => {
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::AnyStart,
                    got: other?,
                })
            }
        }
    }
}

pub fn expect_opening_of<BR, XR>(
    xml: &mut XR,
    expected: &str,
) -> Result<StartElementWrapper, XmlReadingError>
where
    BR: BufRead,
    XR: XmlReader<BR>,
{
    loop {
        match xml.next()? {
            XmlEvent::Whitespace(_) => { /* whitespace is the reason we want to loop */ }
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                return if name.local_name == expected {
                    Ok(StartElementWrapper::new(
                        name,
                        attributes,
                        namespace.clone(),
                    ))
                } else {
                    Err(XmlReadingError::UnexpectedEvent {
                        expected: ExpectedXmlEvent::Start(expected.to_string()),
                        got: XmlEvent::StartElement {
                            name,
                            attributes,
                            namespace,
                        }, // this is retarded but could not figure any better way
                    })
                };
            }
            other => {
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::AnyStart,
                    got: other,
                })
            }
        }
    }
}

/// reads the next event from the xml reader, returns Ok(()) if it is an element with the
/// specified name, otherwise returns an error.
/// since the closing tag only contains the name, returning `()` in the Ok variant is enough
pub fn expect_closure_of<BR, XR>(xml: &mut XR, expected: &str) -> Result<(), XmlReadingError>
where
    BR: BufRead,
    XR: XmlReader<BR>,
{
    loop {
        match xml.next()? {
            XmlEvent::Whitespace(_) => { /* whitespace is the reason we want to loop */ }
            XmlEvent::EndElement { name } => {
                return if name.local_name == expected {
                    Ok(())
                } else {
                    Err(XmlReadingError::UnexpectedEvent {
                        expected: ExpectedXmlEvent::End(expected.to_string()),
                        got: XmlEvent::EndElement { name },
                    })
                };
            }
            other => {
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::AnyEnd,
                    got: other,
                })
            }
        }
    }
}

/// maps xml items in xml list into a vector of items by applying the processing function
/// to each xml item in the xml list. Expects the xml reader to be at the opening tag of the list.
/// Returns the vector of items and leaves the xml reader at the closing tag of the list.
/// Returns error if any tags other than opening tags of `item_name` or closing tag of `list_name`
/// are encountered, or if the processing function returns an error.
/// @param xml - xml reader
/// @param list_name - name of the list element
/// @param item_name - name of the item element
/// @param processing_fn - function that processes the item element into the item. `map_list`
/// hands the control to this function with `xml` set to the start of the item element.
/// `processing_fn` is expected to return the item and leave the xml reader at the closing tag
/// of the item element.
pub fn map_list<XR, BR, F, I>(
    xml: &mut XR,
    list_name: &str,
    item_name: &str,
    processing_fn: F,
) -> Result<Vec<I>, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
    F: Fn(&mut XR, StartElementWrapper) -> Result<I, XmlReadingError>,
{
    let mut acc = Vec::<I>::new();

    loop {
        match xml.next()? {
            XmlEvent::Whitespace(_) => { /* whitespace is the reason we want to loop */ }

            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                if name.local_name == item_name {
                    acc.push(processing_fn(
                        xml,
                        StartElementWrapper::new(name, attributes, namespace),
                    )?);
                    continue;
                }
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::Start(item_name.to_string()),
                    got: XmlEvent::StartElement {
                        name,
                        attributes,
                        namespace,
                    },
                });
            }

            XmlEvent::EndElement { name } => {
                if name.local_name == list_name {
                    return Ok(acc);
                }

                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::End(list_name.to_string()),
                    got: XmlEvent::EndElement { name },
                });
            }

            other => {
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::AnyStart,
                    got: other,
                })
            }
        }
    }
}

/// iterates through the xml until it finds the first opening tag with the given name
/// (specifically, opening_element.name.local_name == expected_name)
pub fn find_start_of<XR, BR>(xml: &mut XR, expected_name: &str) -> Result<(), XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
{
    loop {
        match xml.next()? {
            xml::reader::XmlEvent::StartElement { name: n, .. }
                if n.local_name == expected_name =>
            {
                return Ok(())
            }
            xml::reader::XmlEvent::EndDocument => {
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::Start(expected_name.into()),
                    got: XmlEvent::EndDocument,
                })
            }
            _ => continue, // should be uninteresting
        }
    }
}

// todo this one is likely useless
/// Iterates through the xml until it finds the closing tag with the given name,
/// specifically, closing_element.name.local_name == expected_name.
///
/// Is also capable of working with recursive elements (elements that can contain themselves).
/// In that case, this function returns once it encounters the closing tag of the element
/// it is called from.
pub fn consume_the_rest_of_element<XR, BR>(
    xml: &mut XR,
    element_name: &str,
) -> Result<(), XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
{
    let mut depth = 0usize;
    loop {
        match xml.next()? {
            xml::reader::XmlEvent::StartElement { name: n, .. } if n.local_name == element_name => {
                depth += 1;
            }
            xml::reader::XmlEvent::EndElement { name: n } if n.local_name == element_name => {
                if depth == 0 {
                    return Ok(());
                } else {
                    depth -= 1;
                }
            }
            xml::reader::XmlEvent::EndDocument => {
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: ExpectedXmlEvent::End(element_name.into()),
                    got: XmlEvent::EndDocument,
                })
            }
            _ => continue,
        }
    }
}
