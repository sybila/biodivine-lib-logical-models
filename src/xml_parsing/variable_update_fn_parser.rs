#![allow(dead_code)] // todo remove

use std::{
    collections::{HashMap, HashSet},
    io::BufRead,
    str::FromStr,
};

use xml::reader::XmlEvent;

use crate::{
    expression_components::expression::Expression,
    system::variable_update_function::UnprocessedVariableUpdateFn,
};

use super::{
    utils::expect_opening,
    utils::{expect_closure_of, expect_opening_of, map_list, StartElementWrapper, XmlReadingError},
    xml_reader::XmlReader,
};

impl<T> UnprocessedVariableUpdateFn<T>
where
    T: FromStr,
{
    /// Parses the <transition> XML element into a VariableUpdateFn struct.
    /// Expects the parameter `xml` to be at the start of the <transition> XML element.
    pub fn try_from_xml<XR, BR>(xml: &mut XR) -> Result<Self, XmlReadingError>
    where
        XR: XmlReader<BR>,
        BR: BufRead,
        T: FromStr,
    {
        let some_start_element = expect_opening(xml)?;
        if !matches!(
            some_start_element.name.local_name.as_str(),
            "listOfInputs" | "listOfOutputs"
        ) {
            return Err(XmlReadingError::UnexpectedEvent {
                expected: super::utils::ExpectedXmlEvent::Start(
                    "listOfInputs or listOfOutputs".to_string(),
                ),
                got: XmlEvent::StartElement {
                    name: some_start_element.name,
                    attributes: some_start_element.attributes,
                    namespace: some_start_element.namespace,
                },
            });
        }

        // listOfInputs may or may not be present - either case is accepted
        let input_vars_names = if some_start_element.name.local_name == "listOfInputs" {
            let aux = map_list(xml, "listOfInputs", "input", process_input_var_name_item)?;
            expect_opening_of(xml, "listOfOutputs")?; // must be followed by listOfOutputs
            aux
        } else {
            Vec::new()
        };

        let target_vars_names =
            map_list(xml, "listOfOutputs", "output", process_output_var_name_item)?;
        let target_variable_name = match target_vars_names.as_slice() {
            [single_target_variable_name] => single_target_variable_name.clone(),
            _ => {
                return Err(XmlReadingError::WrongAmountOfElements {
                    expected_amount: 1,
                    found_items_string: target_vars_names.join(", "),
                })
            }
        };

        expect_opening_of(xml, "listOfFunctionTerms")?;
        let (default, terms) = get_default_and_list_of_terms(xml)?;

        expect_closure_of(xml, "transition")?;

        Ok(UnprocessedVariableUpdateFn::new(
            input_vars_names,
            target_variable_name,
            terms,
            default,
        ))
    }
}

fn process_input_var_name_item<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    current: StartElementWrapper,
) -> Result<String, XmlReadingError> {
    let mut qualitative_species = current.attributes.iter().filter_map(|att| {
        if att.name.local_name == "qualitativeSpecies" {
            Some(att.value.clone())
        } else {
            None
        }
    });

    let item = qualitative_species
        .next()
        .ok_or(XmlReadingError::NoSuchAttribute(
            "qualitativeSpecies".to_string(),
        ))?; // todo

    expect_closure_of(xml, "input")?;

    Ok(item)
}

fn process_output_var_name_item<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    current: StartElementWrapper,
) -> Result<String, XmlReadingError> {
    let mut qualitative_species = current.attributes.iter().filter_map(|att| {
        if att.name.local_name == "qualitativeSpecies" {
            Some(att.value.clone())
        } else {
            None
        }
    });

    let item = qualitative_species
        .next()
        .ok_or(XmlReadingError::NoSuchAttribute(
            "value after qualitativeSpecies".to_string(),
        ))?;

    expect_closure_of(xml, "output")?;

    Ok(item)
}

type Out<T> = (T, Vec<(T, Expression<T>)>);

fn get_default_and_list_of_terms<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
) -> Result<Out<T>, XmlReadingError> {
    let default_element = expect_opening_of(xml, "defaultTerm")?;

    let default_val = result_level_from_attributes(&default_element)?;

    expect_closure_of(xml, "defaultTerm")?;

    // expect_opening_of("listOfFunctionTerms", xml)?; // already inside "listOfFunctionTerms"; first item was the default element
    let values_and_expressions = map_list(
        xml,
        "listOfFunctionTerms",
        "functionTerm",
        |xml, current| process_function_term_item(xml, &current),
    )?;

    Ok((default_val, values_and_expressions))
}

fn process_function_term_item<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    current: &StartElementWrapper,
) -> Result<(T, Expression<T>), XmlReadingError> {
    let res_lvl = result_level_from_attributes(current)?;

    expect_opening_of(xml, "math")?;

    expect_opening_of(xml, "apply")?; // open this tag to satisfy `Expression::try_from_xml`s precondition
    let exp = Expression::try_from_xml(xml)?;

    expect_closure_of(xml, "math")?;
    expect_closure_of(xml, "functionTerm")?;

    Ok((res_lvl, exp))
}

fn result_level_from_attributes<T: FromStr>(
    elem: &StartElementWrapper,
) -> Result<T, XmlReadingError> {
    let attribute_with_result_lvl = elem
        .attributes
        .iter()
        .find(|attr_name| attr_name.name.local_name == "resultLevel")
        .ok_or(XmlReadingError::NoSuchAttribute("resultLevel".to_string()))?;

    attribute_with_result_lvl
        .value
        .parse::<T>()
        .map_err(|_| XmlReadingError::ParsingError(attribute_with_result_lvl.value.clone()))
}

// todo alright i have no idea how to do this
/*
// fn load_from_sbml<XR, BR, T>(xml: &mut XR) -> Result<> {
//     // todo
//     // skip until <listOfFunctionTerms>
//     // then call load_all_update_fns
// }

fn load_from_sbml<XR, BR, T>(
    file_path: &str,
) -> Result<HashMap<String, UnprocessedVariableUpdateFn<T>>, XmlReadingError>
where
    XR: XmlReader<std::io::BufRead>,
    BR: BufRead,
    // XR: XmlReader<std::io::BufRead<std::fs::File>>,
    T: FromStr + Default,
{
    // let xd = std::io::BufReader::new(
    //     std::fs::File::open(file_path).expect("Could not open file for reading"),
    // );
    // let xd = xml::reader::EventReader::new(xd);
    // // let mut xml = XR::new(super::xml_reader::XmlReader::new(xd));
    // let mut xml = XR::new(xd);

    // let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
    //     std::fs::File::open(file_path).unwrap(),
    // ));

    // let xml = xml::reader::EventReader::new(std::io::BufReader::new(
    //     std::fs::File::open(file_path).unwrap(),
    // ));

    // let mut xml = XR::new(xml);

    // super::utils::find_start_of(&mut xml, "listOfTransitions")?;

    // load_all_update_fns(&mut xml)

    todo!()
}

/// Expects `xml` to be at the start of an sbml file.
/// Loads all <functionTerm> elements into a HashMap.
fn load_from_sbml_buf_reader<XR, BR, T>(
    // xml: &mut XR,
    // file_path: &str,
    buf_reader: BR,
) -> Result<HashMap<String, UnprocessedVariableUpdateFn<T>>, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
    T: FromStr + Default,
{
    // let xd = std::io::BufReader::new(
    //     std::fs::File::open(file_path).expect("Could not open file for reading"),
    // );
    // let xd = xml::reader::EventReader::new(xd);
    // // let mut xml = XR::new(super::xml_reader::XmlReader::new(xd));
    // let mut xml = XR::new(xd);

    // let mut xml = xml::reader::EventReader::new(std::io::BufReader::new(
    //     std::fs::File::open(file_path).unwrap(),
    // ));

    let xml = xml::reader::EventReader::new(buf_reader);

    let mut xml = XR::new(xml);

    super::utils::find_start_of(&mut xml, "listOfTransitions")?;

    load_all_update_fns(&mut xml)
}
*/

/// Expect the current XML element to be <listOfFunctionTerms>
/// Loads all contained <functionTerm> elements into a HashMap.
pub fn load_all_update_fns<XR, BR, T>(
    xml: &mut XR,
) -> Result<HashMap<String, UnprocessedVariableUpdateFn<T>>, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
    T: FromStr + Default,
{
    let vars_and_their_update_fns = map_list(
        xml,
        "listOfTransitions",
        "transition",
        |xml, _start_element| UnprocessedVariableUpdateFn::<T>::try_from_xml(xml),
    )?
    .into_iter()
    .map(|update_fn| (update_fn.target_var_name.clone(), update_fn))
    .collect::<HashMap<_, _>>();

    let vars_possibly_without_update_fns = vars_and_their_update_fns
        .values()
        .flat_map(|update_fn| update_fn.input_vars_names.clone())
        .collect::<HashSet<_>>();

    let all_vars_and_their_update_fns = vars_possibly_without_update_fns.into_iter().fold(
        vars_and_their_update_fns,
        |mut acc, update_fn_name| {
            acc.entry(update_fn_name.clone()).or_insert_with(|| {
                UnprocessedVariableUpdateFn::<T>::new(
                    Vec::new(),
                    update_fn_name,
                    Vec::new(),
                    Default::default(),
                )
            });
            acc
        },
    );

    Ok(all_vars_and_their_update_fns)
}
