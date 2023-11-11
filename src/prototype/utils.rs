use biodivine_lib_bdd::{Bdd, BddPartialValuation, BddVariable};
use std::{collections::HashMap, io::BufRead, str::FromStr};
use std::fmt::Debug;
use std::ops::Shr;
use num_bigint::BigInt;
use xml::{
    attribute::OwnedAttribute,
    name::OwnedName,
    namespace::Namespace,
    reader::{EventReader, XmlEvent},
};

use crate::{SmartSystemUpdateFn, SymbolicDomain, UpdateFn};

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

/// use another XMLReader implementation from this file to get the update functions - the thing you need to pass to DebuggingReader::new
/// this is used to go through the xml (the same one that was previously loaded into update_fns) and
/// print error messages wherever a variable is compared to a value higher than its update_fn allows it to be
/// and also if there is a variable name, which is not known (does not have an update fn)
pub struct DebuggingReader<BR: BufRead> {
    loud_xml: LoudReader<BR>,
    vars_and_their_max_values: HashMap<String, u8>,
    complain_about_values_too_large: bool,
    complain_about_unknown_variable_name: bool,
    current_ci: Option<String>, // this is just a retarded way of holding the context of what variable is ananlyzed/compared with value
    expecting_variable_name: bool,
    expecting_variable_value: bool,
}

impl<BR: BufRead> DebuggingReader<BR> {
    pub fn new(
        xml: EventReader<BR>,
        update_fns: &HashMap<String, UpdateFn<u8>>,
        complain_about_values_too_large: bool,
        complain_about_unknown_variable_name: bool,
    ) -> Self {
        let vars_and_their_max_values = update_fns
            .iter()
            .map(|(var_name, update_fn)| {
                let max_value_this_variable_can_get_according_to_its_update_fn = update_fn
                    .terms
                    .iter()
                    .map(|(val, _condition)| val)
                    .chain(std::iter::once(&update_fn.default))
                    .max()
                    .unwrap()
                    .to_owned();

                (
                    var_name.to_owned(),
                    max_value_this_variable_can_get_according_to_its_update_fn,
                )
            })
            .collect::<HashMap<_, _>>();

        Self {
            loud_xml: LoudReader::new(xml),
            vars_and_their_max_values,
            complain_about_values_too_large,
            complain_about_unknown_variable_name,
            current_ci: None,
            expecting_variable_name: false,
            expecting_variable_value: false,
        }
    }
}

impl<BR: BufRead> XmlReader<BR> for DebuggingReader<BR> {
    fn next(&mut self) -> Result<XmlEvent, String> {
        match self.loud_xml.next() {
            Ok(e) => {
                match e.clone() {
                    XmlEvent::StartElement {
                        name,
                        // attributes,
                        // namespace,
                        ..
                    } => match name.local_name.as_str() {
                        "ci" => self.expecting_variable_name = true,
                        "cn" => self.expecting_variable_value = true,
                        _ => {}
                    },
                    XmlEvent::Characters(content) => {
                        if self.expecting_variable_name {
                            self.expecting_variable_name = false;
                            self.current_ci = Some(content.to_string());
                        }

                        if self.expecting_variable_value {
                            self.expecting_variable_value = false;
                            let actual_value = content.clone().trim().parse::<u8>().unwrap_or_else(
                                |_| panic!(
                                            "currently only allowing DebugReader parse u8 values; got {}",
                                            content
                                )
                            );

                            let associated_max_value = self.vars_and_their_max_values.get(
                                &self
                                    .current_ci
                                    .clone()
                                    .expect("current_ci should be initialized"),
                            );

                            match associated_max_value {
                                None => {
                                    if self.complain_about_unknown_variable_name {
                                        eprintln!(
                                            "[debug: UNKNOWN_VARIABLE] got variable with name {} in this proposition, but no such name known; known names are {:?}",
                                            self.current_ci.clone().unwrap(),
                                            self.vars_and_their_max_values.keys()
                                        )
                                    }
                                }
                                Some(expected_max_value) => {
                                    if *expected_max_value < actual_value
                                        && self.complain_about_values_too_large
                                    {
                                        eprintln!(
                                            "[debug: VALUE_TOO_BIG] comparing variable {} (whose domain is only [0, {}]) with value {}",
                                            self.current_ci.clone().unwrap(),
                                            expected_max_value,
                                            actual_value
                                        )
                                    }
                                }
                            }
                        }
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

// impl<BR: BufRead> XmlReader<BR> for DebuggingReader<BR> {
//     fn next(&mut self) -> Result<XmlEvent, String> {}
// }

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
                    XmlEvent::StartElement { .. } => {
                        self.curr_line += 1;
                    }
                    XmlEvent::EndElement { .. } => {
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

pub fn find_bdd_variables_prime<D: SymbolicDomain<T>, T>(
    target_variable: &BddVariable,
    target_sym_dom: &D,
    target_sym_dom_primed: &D,
) -> BddVariable {
    target_sym_dom
        .symbolic_variables()
        .into_iter()
        .zip(target_sym_dom_primed.symbolic_variables())
        .find_map(|(maybe_target_variable, its_primed)| {
            if maybe_target_variable == *target_variable {
                Some(its_primed)
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // this might happen if the supplied target_sym_domain does not contain target_variable
            // or if the target_sym_dom_primed has less elements (but this fn should not be called with such)
            panic!(
                "did not find the variable {} in given target sym domain",
                target_variable,
            )
        })
}


/// Compute a [Bdd] which represents a single (un-primed) state within the given symbolic `set`.
pub fn pick_state_bdd<D: SymbolicDomain<u8> + Debug>(system: &SmartSystemUpdateFn<D, u8>, set: &Bdd) -> Bdd {
    // Unfortunately, this is now a bit more complicated than it needs to be, because
    // we have to ignore the primed variables, but it shouldn't bottleneck anything outside of
    // truly extreme cases.
    let standard_variables = system.standard_variables();
    let valuation = set.sat_witness()
        .expect("Cannot pick state from an empty set.");
    let mut state_data = BddPartialValuation::empty();
    for var in standard_variables {
        state_data.set_value(var, valuation.value(var))
    }
    system.get_bdd_variable_set().mk_conjunctive_clause(&state_data)
}


/// Pick a state from a symbolic set and "decode" it into normal integers.
pub fn pick_state_map<D: SymbolicDomain<u8> + Debug>(system: &SmartSystemUpdateFn<D, u8>, set: &Bdd) -> HashMap<String, u8> {
    let valuation = set.sat_witness()
        .expect("The set is empty.");
    let valuation = BddPartialValuation::from(valuation);
    let mut result = HashMap::new();
    for var in system.get_system_variables() {
        let Some(domain) = system.named_symbolic_domains.get(&var) else {
            unreachable!("Variable exists but has no symbolic domain.")
        };
        let value = domain.decode_bits(&valuation);
        result.insert(var, value);
    }
    result
}

/// Encode a "state" (assignment of integer values to all variables) into a [Bdd] that is valid
/// within the provided [SmartSystemUpdateFn].
pub fn encode_state_map<D: SymbolicDomain<u8> + Debug>(system: &SmartSystemUpdateFn<D, u8>, state: &HashMap<String, u8>) -> Bdd {
    let mut result  = BddPartialValuation::empty();
    for var in system.get_system_variables() {
        let Some(value) = state.get(&var) else {
            panic!("Value for {var} missing.");
        };
        let Some(domain) = system.named_symbolic_domains.get(&var) else {
            unreachable!("Variable exists but has no symbolic domain.")
        };
        domain.encode_bits(&mut result, value);
    }
    system.get_bdd_variable_set().mk_conjunctive_clause(&result)
}

pub fn log_percent(set: &Bdd, universe: &Bdd) -> f64 {
    set.cardinality().log2() / universe.cardinality().log2() * 100.0
}

/// Compute an (approximate) count of state in the given `set` using the encoding of `system`.
pub fn count_states<D: SymbolicDomain<u8> + Debug>(system: &SmartSystemUpdateFn<D, u8>, set: &Bdd) -> f64 {
    let symbolic_var_count = system.get_bdd_variable_set().num_vars() as i32;
    // TODO:
    //   Here we assume that exactly half of the variables are primed, which may not be true
    //   in the future, but should be good enough for now.
    assert_eq!(symbolic_var_count % 2, 0);
    let primed_vars = symbolic_var_count / 2;
    set.cardinality() / 2.0f64.powi(primed_vars)
}

/// Same as [count_states], but with exact unbounded integers.
pub fn count_states_exact<D: SymbolicDomain<u8> + Debug>(system: &SmartSystemUpdateFn<D, u8>, set: &Bdd) -> BigInt {
    let symbolic_var_count = system.get_bdd_variable_set().num_vars() as i32;
    // TODO:
    //   Here we assume that exactly half of the variables are primed, which may not be true
    //   in the future, but should be good enough for now.
    assert_eq!(symbolic_var_count % 2, 0);
    let primed_vars = symbolic_var_count / 2;
    set.exact_cardinality().shr(primed_vars)
}