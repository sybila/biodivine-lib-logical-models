// todo how to work with the "variables" that are not mentioned in the listOfTransitions?

use std::{collections::HashMap, io::BufRead};

use biodivine_lib_bdd::BddVariableSetBuilder;
use xml::EventReader;

use crate::{
    SymbolicDomain, UnaryIntegerDomain, UpdateFn, UpdateFnBdd, VariableUpdateFnCompiled, XmlReader,
};

struct SystemUpdateFn<D: SymbolicDomain<T>, T> {
    penis: std::marker::PhantomData<D>,
    penis_the_second: std::marker::PhantomData<T>,
    pub update_fns: HashMap<String, VariableUpdateFnCompiled<UnaryIntegerDomain, u8>>,
}

impl<D: SymbolicDomain<u8>> SystemUpdateFn<D, u8> {
    /// expects the xml reader to be at the start of the <listOfTransitions> element
    pub fn try_from_xml<XR: XmlReader<BR>, BR: BufRead>(
        xml: &mut XR,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let var_names_and_upd_fns = load_all_update_fns(xml)?;
        let ctx = vars_and_their_max_values(&var_names_and_upd_fns);

        let mut bdd_variable_set_builder = BddVariableSetBuilder::new();
        let named_symbolic_domains = ctx
            .into_iter()
            .map(|(name, max_value)| {
                (
                    name.clone(),
                    UnaryIntegerDomain::new(
                        &mut bdd_variable_set_builder,
                        &name,
                        max_value.to_owned(),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let variable_set = bdd_variable_set_builder.build();

        let update_fns = var_names_and_upd_fns
            .into_values()
            .map(|update_fn| {
                (
                    update_fn.target_var_name.clone(),
                    UpdateFnBdd::from_update_fn(update_fn, &variable_set, &named_symbolic_domains)
                        .into(),
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(Self {
            update_fns,
            penis: std::marker::PhantomData,
            penis_the_second: std::marker::PhantomData,
        })
    }
}

#[allow(dead_code)]
/// expects the xml reader to be at the start of the <listOfTransitions> element
fn load_all_update_fns<XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
    // todo generic
) -> Result<HashMap<String, UpdateFn<u8>>, Box<dyn std::error::Error>> {
    Ok(crate::process_list(
        "listOfTransitions",
        "transition",
        |xml, _unused_opening_tag| UpdateFn::<u8>::try_from_xml(xml),
        xml,
    )?
    // todo this might not be the smartest nor useful; the name is already in the fn
    //  but this will allow us to access the appropriate fn quickly
    .into_iter()
    .map(|upd_fn| (upd_fn.target_var_name.clone(), upd_fn))
    .collect())
}

// todo this might not be the best way; it cannot detect that some values are unreachable;
// todo  for example:
//     term0: output = 1 if (const true)
//     default: output = 9999
// todo this will be detected as max value = 9999, but in reality it is 1
// todo; -> warnings abt unreachable terms important during conversion to CompiledUpdateFn
/// returns a map of variable names and their max values
fn vars_and_their_max_values(
    vars_and_their_upd_fns: &HashMap<String, UpdateFn<u8>>,
) -> HashMap<String, u8> {
    vars_and_their_upd_fns
        .iter()
        .map(|(var_name, upd_fn)| {
            (
                var_name.clone(),
                upd_fn
                    .terms
                    .iter()
                    .map(|(val, _)| val)
                    .chain(std::iter::once(&upd_fn.default))
                    .max()
                    .unwrap()
                    .to_owned(),
            )
        })
        .collect()
}

// todo also maybe just zip the two maps together
fn compile_update_fns(
    _var_names_and_upd_fns: &HashMap<String, UpdateFn<u8>>,
    _ctx: &HashMap<String, u8>,
) -> HashMap<String, Vec<u8>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use crate::UnaryIntegerDomain;

    use super::SystemUpdateFn;

    #[test]
    fn test() {
        let file = std::fs::File::open("data/dataset.sbml").expect("cannot open file");
        let br = std::io::BufReader::new(file);

        let reader = xml::reader::EventReader::new(br);
        let mut reader = crate::LoudReader::new(reader); // uncomment to see how xml is loaded

        crate::find_start_of(&mut reader, "listOfTransitions").expect("cannot find start of list");
        let _system_update_fn: SystemUpdateFn<UnaryIntegerDomain, u8> =
            super::SystemUpdateFn::try_from_xml(&mut reader).unwrap();
    }
}
