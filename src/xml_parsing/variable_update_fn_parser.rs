use std::{io::BufRead, str::FromStr};

use xml::reader::XmlEvent;

use crate::{
    expression_components::expression::Expression,
    system::variable_update_function::VariableUpdateFn,
};

use super::{
    utils::expect_opening,
    utils::{expect_closure_of, expect_opening_of, map_list, StartElementWrapper, XmlReadingError},
    xml_reader::XmlReader,
};

/// expects the xml reader to be at the start of the <transition> element
impl<T: FromStr> VariableUpdateFn<T> {
    pub fn try_from_xml<XR: XmlReader<BR>, BR: BufRead>(
        xml: &mut XR,
    ) -> Result<Self, XmlReadingError> {
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

        // listOfInputs might not be present at all
        let input_vars_names = if some_start_element.name.local_name == "listOfInputs" {
            let aux = map_list(xml, "listOfInputs", "input", process_input_var_name_item)?;
            expect_closure_of(xml, "listOfOutputs")?; // must be followed by listOfOutputs
            aux
        } else {
            Vec::new()
        };

        // listOfOutputs must be present
        let target_vars_names =
            map_list(xml, "listOfOutputs", "output", process_output_var_name_item)?;
        let mut target_vars_names = target_vars_names.iter();
        let head = target_vars_names // todo head must be used; caller has no way of knowing which variable is the target otherwise
            .next()
            .ok_or(XmlReadingError::UnexpectedEvent {
                expected: super::utils::ExpectedXmlEvent::Start("target var name".to_string()),
                got: XmlEvent::EndElement {
                    name: some_start_element.name,
                },
            });
        target_vars_names.next().map_or_else(
            || Ok::<(), ()>(()),
            |_elem| {
                // Err(XmlReadingError::UnexpectedEvent {
                //     expected: super::utils::ExpectedXmlEvent::End("list of vars names".to_string()),
                //     got: // ...,
                // })
                panic!("expected only one target var but found multiple; todo might want to change this")
            },
        );
        // .ok_or(XmlReadingError::UnexpectedEvent { expected: super::utils::ExpectedXmlEvent::End("listOfOutputs".to_string()), got: () })

        expect_opening_of(xml, "listOfFunctionTerms")?;
        let (default, terms) = get_default_and_list_of_terms(xml)?;

        expect_closure_of(xml, "transition")?;
        // Ok(Self::new(input_vars_names, head.into(), terms, default))
        // Ok(VariableUpdateFn::new(terms, default))

        let xd = VariableUpdateFn::new(terms, default);

        Ok(xd)
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
        // .ok_or("expected \"qualitativeSpecies\" arg in input, but none found")?;
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
    // firs should be the default
    let default_element = expect_opening_of(xml, "defaultTerm")?;

    let default_val = result_level_from_attributes(&default_element)?;

    expect_closure_of(xml, "defaultTerm")?;

    // expect_opening_of("functionTerms", xml)?; // already inside "functionTerms" List; first item was default element
    let values_and_expressions = map_list(
        xml,
        "listOfFunctionTerms",
        "functionTerm",
        process_function_term_item,
    )?;

    Ok((default_val, values_and_expressions))
}

fn process_function_term_item<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    current: StartElementWrapper,
) -> Result<(T, Expression<T>), XmlReadingError> {
    let res_lvl = result_level_from_attributes(&current)?;

    expect_opening_of(xml, "math")?;
    // try_from_xml expects to have the first apply already opened
    expect_opening_of(xml, "apply")?;

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
        .map_err(|_| XmlReadingError::ParsingError(attribute_with_result_lvl.value))
}
