use std::io::BufRead;
use xml::{
    attribute::OwnedAttribute,
    name::OwnedName,
    namespace::Namespace,
    reader::{EventReader, XmlEvent},
};

pub fn expect_opening<T: BufRead>(
    xml: &mut EventReader<T>,
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

pub fn expect_opening_of<T: BufRead>(
    expected: &str,
    xml: &mut EventReader<T>,
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
pub fn expect_closure_of<T: BufRead>(
    expected: &str,
    xml: &mut EventReader<T>,
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
pub fn process_list<T: BufRead, Fun, Res>(
    list_name: &str,
    item_name: &str,
    processing_fn: Fun,
    xml: &mut EventReader<T>,
) -> Result<Vec<Res>, Box<dyn std::error::Error>>
where
    Fun: Fn(&mut EventReader<T>, StartElementWrapper) -> Result<Res, Box<dyn std::error::Error>>,
{
    println!(
        "inside processing list with args: {}, {}",
        list_name, item_name
    );
    let mut acc = Vec::<Res>::new();

    // hope this ll work

    loop {
        let elem = xml.next();
        println!("elem: {:?}", elem);
        match elem {
            Ok(XmlEvent::Whitespace(_)) => { /* ignore */ }
            Ok(XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            }) => {
                if name.local_name == item_name {
                    println!("processing item {}", item_name);
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
