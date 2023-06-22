use super::expression::Expression;
use std::io::BufRead;
use xml::reader::EventReader;

/// represents collection of tuples of the result values and the associated conditions. there is also the default value.
/// todo think about how the functions should be evaluated - should we allow the conditions to "overlap" and say that the first one counts?
/// (would not be hard to implement, just (!all_previous && current); the default would then be analogically (!all_previous && true)).
/// in that case, the !all_previous should be somehow cached and passed to the next ofc
pub struct UpdateFn {
    pub target_var_name: String,
    // todo should likely be in bdd repr already;
    // that should be done for the intermediate repr of Expression as well;
    // will do that once i can parse the whole xml
    pub cases: Vec<(f64, Expression)>,
    pub default: f64,
}

impl UpdateFn {
    pub fn try_from_xml<T: BufRead>(
        xml: &mut EventReader<T>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // startin with the xml.current() on the `<qual:transition qual:id="target_var_name">` element

        todo!()
    }
}
