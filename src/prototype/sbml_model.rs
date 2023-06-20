use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str, to_string}; // todo likely want to read from stream or smth

struct Apply {}

// this is kinda scuffed but i need to preserve the order of the elements while allowing
// different permutations of the elements
// since serde_xml_rs does not work well with different orderings of the children, this
// will have to do; the correct number & type of children will have to be checked manually elsewhere
#[derive(Debug, Deserialize, Serialize)]
struct Proposition {
    events: Vec<PropositionEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
enum PropositionEvent {
    CmpOp(CmpOp),
    Ci(Ci),
    Cn(Cn),
}

/// represents variable name in apply terminal
#[derive(Debug, Deserialize, Serialize)]
struct Ci {
    #[serde(rename = "$value")]
    // renaming so that it is wrapped in the tag instead of being tags attribute
    value: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Cn {
    value: f64, // todo
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum CmpOp {
    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,
}

pub fn trying() {
    let lol = Proposition {
        events: vec![
            PropositionEvent::CmpOp(CmpOp::Eq),
            PropositionEvent::Ci(Ci {
                value: "x".to_string(),
            }),
            PropositionEvent::Cn(Cn { value: 5.0 }),
        ],
    };

    let xml = to_string(&lol).unwrap();
    println!("{}", xml);
}
