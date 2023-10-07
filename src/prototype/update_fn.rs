use crate::{expect_closure_of, expect_opening_of, process_list, StartElementWrapper, XmlReader};

use super::expression::Expression;
use super::utils::expect_opening;
use std::{io::BufRead, str::FromStr};
use xml::reader::EventReader;

/// represents collection of tuples of the result values and the associated conditions. there is also the default value.
/// todo think about how the functions should be evaluated - should we allow the conditions to "overlap" and say that the first one counts?
/// (would not be hard to implement, just (!all_previous && current); the default would then be analogically (!all_previous && true)).
/// in that case, the !all_previous should be somehow cached and passed to the next ofc
#[derive(Debug)]
pub struct UpdateFn<T> {
    pub input_vars_names: Vec<String>,
    pub target_var_name: String,
    // todo should likely be in bdd repr already;
    // that should be done for the intermediate repr of Expression as well;
    // will do that once i can parse the whole xml
    pub terms: Vec<(T, Expression<T>)>,
    pub default: T,
}

impl<T> UpdateFn<T> {
    pub fn new(
        input_vars_names: Vec<String>,
        target_var_name: String,
        terms: Vec<(T, Expression<T>)>,
        default: T,
    ) -> Self {
        Self {
            input_vars_names,
            target_var_name,
            terms,
            default,
        }
    }
}

/// expects the xml reader to be at the start of the <transition> element
impl<T: FromStr> UpdateFn<T> {
    pub fn try_from_xml<XR: XmlReader<BR>, BR: BufRead>(
        xml: &mut XR,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let some_start_element = expect_opening(xml)?;
        if !matches!(
            some_start_element.name.local_name.as_str(),
            "listOfInputs" | "listOfOutputs"
        ) {
            return Err(format!(
                "expected either listOfInputs or listOfOutputs, got {}",
                some_start_element.name.local_name
            )
            .into());
        }

        // listOfInputs might not be present at all
        let input_vars_names = if some_start_element.name.local_name == "listOfInputs" {
            let aux = process_list("listOfInputs", "input", process_input_var_name_item, xml)?;
            expect_opening_of("listOfOutputs", xml)?; // must be followed by listOfOutputs
            aux
        } else {
            Vec::new()
        };

        // listOfOutputs must be present
        // todo want to generalize this to list of outputs in the future
        // maybe would make sense to use iterators? i do not see too big gain tho
        let target_vars_names =
            process_list("listOfOutputs", "output", process_output_var_name_item, xml)?;
        let mut target_vars_names = target_vars_names.iter(); // lmao idk
        let head = target_vars_names
            .next()
            .ok_or("expected target variable name but none found")?;
        target_vars_names
            .next()
            .map_or_else(|| Ok(()), |_| Err("expected only one target var but found multiple; todo might want to change this"))?;

        expect_opening_of("listOfFunctionTerms", xml)?;
        let (default, terms) = get_default_and_list_of_terms(xml)?;

        expect_closure_of("transition", xml)?;
        Ok(Self::new(input_vars_names, head.into(), terms, default))
    }
}

fn process_input_var_name_item<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    current: StartElementWrapper,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut qualitative_species = current.attributes.iter().filter_map(|att| {
        if att.name.local_name == "qualitativeSpecies" {
            Some(att.value.clone())
        } else {
            None
        }
    });

    let item = qualitative_species
        .next()
        .ok_or("expected \"qualitativeSpecies\" arg in input, but none found")?;

    expect_closure_of("input", xml)?;

    Ok(item)
}

fn process_output_var_name_item<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    current: StartElementWrapper,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut qualitative_species = current.attributes.iter().filter_map(|att| {
        if att.name.local_name == "qualitativeSpecies" {
            Some(att.value.clone())
        } else {
            None
        }
    });

    let item = qualitative_species
        .next()
        .ok_or("expected \"qualitativeSpecies\" arg in output, but none found")?;

    expect_closure_of("output", xml)?;

    Ok(item)
}

fn get_default_and_list_of_terms<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
) -> Result<(T, Vec<(T, Expression<T>)>), Box<dyn std::error::Error>> {
    // firs should be the default
    let default_element = expect_opening_of("defaultTerm", xml)?;
    let default_val = result_level_from_attributes(&default_element)
        .ok_or("expected \"resultLevel\" with numeric argument in defaultTerm but none found")?;

    expect_closure_of("defaultTerm", xml)?;

    // expect_opening_of("functionTerms", xml)?; // already inside "functionTerms" List; first item was default element
    let values_and_expressions = process_list(
        "listOfFunctionTerms",
        "functionTerm",
        process_function_term_item,
        xml,
    )?;

    Ok((default_val, values_and_expressions))
}

fn process_function_term_item<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    current: StartElementWrapper,
) -> Result<(T, Expression<T>), Box<dyn std::error::Error>> {
    let res_lvl = result_level_from_attributes(&current)
        .ok_or("expected \"resultLevel\" with numeric argument in functionTerm but none found")?;

    expect_opening_of("math", xml)?;
    // try_from_xml expects to have the first apply already opened
    expect_opening_of("apply", xml)?;

    let exp = Expression::try_from_xml(xml)?;

    expect_closure_of("math", xml)?;
    expect_closure_of("functionTerm", xml)?;

    Ok((res_lvl, exp))
}

fn result_level_from_attributes<T: FromStr>(elem: &StartElementWrapper) -> Option<T> {
    elem.attributes.iter().find_map(|attr| {
        if attr.name.local_name == "resultLevel" {
            attr.value.parse::<T>().ok()
        } else {
            None
        }
    })
}
