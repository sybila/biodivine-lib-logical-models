// todo how to work with the "variables" that are not mentioned in the listOfTransitions?

use std::{collections::HashMap, io::BufRead};

use xml::EventReader;

use crate::{UnaryIntegerDomain, UpdateFn, VariableUpdateFnCompiled};

struct SystemUpdateFn {
    pub update_fns: HashMap<String, VariableUpdateFnCompiled<UnaryIntegerDomain, u8>>,
}

impl SystemUpdateFn {
    /// expects the xml reader to be at the start of the <listOfTransitions> element
    pub fn try_from_xml<BR: BufRead>(
        _xml: &mut EventReader<BR>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let var_names_and_upd_fns = load_all_update_fns(_xml)?;
        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        let ctx = vars_and_their_max_values(&var_names_and_upd_fns);

        var_names_and_upd_fns.iter().for_each(|(var_name, upd_fn)| {
            println!(">>>>{}'s updatefn {:?}", var_name, upd_fn);
            println!("{}'s max value {:?}", var_name, ctx.get(var_name));
        });

        todo!()
    }
}

#[allow(dead_code)]
/// expects the xml reader to be at the start of the <listOfTransitions> element
fn load_all_update_fns<BR: BufRead>(
    xml: &mut EventReader<BR>,
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
    #[test]
    fn test() {
        let file = std::fs::File::open("data/dataset.sbml").expect("cannot open file");
        let br = std::io::BufReader::new(file);
        let mut xml = xml::reader::EventReader::new(br);

        crate::find_start_of(&mut xml, "listOfTransitions").expect("cannot find start of list");
        let _system_update_fn = super::SystemUpdateFn::try_from_xml(&mut xml).unwrap();
    }
}
