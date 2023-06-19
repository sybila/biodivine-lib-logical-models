use std::marker::PhantomData;
use thiserror::Error;

use biodivine_lib_bdd as bdd;

/// represents arity of a multi-valued variable in an MVDD.<br>
/// lowest allowed arity is 2 corresponding to a boolean variable.<br>
/// the domain of a variable with arity `n` is `{0, 1, ..., n-1}`.<br>
#[derive(Clone)]
pub struct Arity(u8); // todo decide size; ie eg the original Bdd lib uses u16 iirc

impl Arity {
    /// creates a new Arity with the given arity.<br>
    /// Panics if arity is less than 2.<br>
    pub fn new(arity: u8) -> Self {
        if arity < 2 {
            panic!("Arity must be at least 2, but {} was provided.", arity);
        }

        Self(arity)
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

pub trait MvRepr {
    fn encode(arity: &Arity) -> Vec<bdd::BddVariable>;
}

pub struct MvReprVanHam;
impl MvRepr for MvReprVanHam {
    fn encode(arity: &Arity) -> Vec<bdd::BddVariable> {
        unimplemented!("MvReprVanHam::encode; arity: {}", arity.value())
    }
}

pub struct MvReprGrayCode;
impl MvRepr for MvReprGrayCode {
    fn encode(arity: &Arity) -> Vec<bdd::BddVariable> {
        unimplemented!("MvReprGrayCode::encode; arity: {}", arity.value())
    }
}

pub struct MvddVariable<Repr: MvRepr> {
    repr: PhantomData<Repr>, // til abt PhantomData
    arity: Arity,
    phantom: Vec<bdd::BddVariable>,
}

impl<Repr: MvRepr> MvddVariable<Repr> {
    pub fn new(arity: Arity) -> Self {
        let bdd_repr = Repr::encode(&arity);

        Self {
            repr: PhantomData,
            arity,
            phantom: bdd_repr,
        }
    }

    pub fn arity(&self) -> &Arity {
        &self.arity
    }

    pub fn bdd_repr(&self) -> &Vec<bdd::BddVariable> {
        &self.phantom
    }
}

fn stuff() {
    let arity = Arity::new(3);
    let value = arity.value();
}

struct TermConst(u16);
impl TermConst {
    fn try_from_str(s: &str) -> Result<Self, std::num::ParseIntError> {
        Ok(Self(s.parse::<u16>()?))
    }
    fn value(&self) -> u16 {
        self.0
    }
}
struct TermVar(String);

enum CmpOp {
    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,
}

impl CmpOp {
    fn flip(&self) -> Self {
        match self {
            CmpOp::Eq => CmpOp::Eq,
            CmpOp::Neq => CmpOp::Neq,
            CmpOp::Lt => CmpOp::Gt,
            CmpOp::Leq => CmpOp::Geq,
            CmpOp::Gt => CmpOp::Lt,
            CmpOp::Geq => CmpOp::Leq,
        }
    }

    fn try_from_str(s: &str) -> Result<Self, ParseCmpOpError> {
        match s {
            "eq" => Ok(CmpOp::Eq),
            "neq" => Ok(CmpOp::Neq),
            "lt" => Ok(CmpOp::Lt),
            "leq" => Ok(CmpOp::Leq),
            "gt" => Ok(CmpOp::Gt),
            "geq" => Ok(CmpOp::Geq),
            _ => Err(ParseCmpOpError(s.to_string())),
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid comparison operator: {0}")]
struct ParseCmpOpError(String);

enum Term {
    Flipped(CmpOp, TermConst, TermVar),
    Standard(CmpOp, TermVar, TermConst),
}

// lol this is retarded
#[derive(Debug, Error)]
enum TermError {
    #[error("Invalid comparison operator: {0}")]
    ParseCmpOpError(#[from] ParseCmpOpError),
    #[error("ParseIntError: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl Term {
    // consumes self & returns a flipped version of it
    fn flip(self) -> Self {
        match self {
            Term::Flipped(op, c, v) => Term::Standard(op.flip(), v, c),
            Term::Standard(op, v, c) => Term::Flipped(op.flip(), c, v),
        }
    }

    /// creates somewhat normalized representation of a term from a comparison operator and two strings.<br>
    /// does not guarantee the validity of name of the variable, not even if it is a valid BDD variable name.<br>
    fn try_from_xml_strings(op: String, lhs: String, rhs: String) -> Result<Self, TermError> {
        let op = CmpOp::try_from_str(&op)?;

        if let Ok(lhs_const) = TermConst::try_from_str(&lhs) {
            return Ok(Term::Flipped(op, lhs_const, TermVar(rhs)));
        }

        return Ok(Term::Standard(
            op,
            TermVar(lhs),
            TermConst(rhs.parse::<u16>()?),
        ));
    }
}

// #[derive(Debug)]
// struct TermNode {
//     name: String,
//     attributes: Vec<xml::attribute::OwnedAttribute>,
//     character_data: Option<String>,
// }

// impl TermNode {
//     fn new(name: String, attributes: Vec<xml::attribute::OwnedAttribute>, character_data: Option<String>) -> Self {
//         Self {
//             name,
//             attributes,
//             character_data,
//         }
//     }

//     fn process(&self) {
//         println!("Processing node: {:?}", self);
//         self.attributes.iter().for_each(|attr| {
//             println!("Attribute: {:?}", attr);
//         });
//     }
// }

// fn process_xml_node(node: TermNode) {
//     println!("Processing node: {:?}", node);
// }

// fn literal_from_xml() {
//     unimplemented!();
// }

// use xml::reader::{EventReader, XmlEvent};

// pub fn node_processing() {
//     let xml = r#"
//         <apply>
//             <eq/>
//             <ci> DNAdam </ci>
//             <cn type="integer"> 0 </cn>
//         </apply>
//     "#;

//     let reader = EventReader::new(xml.as_bytes());
//     let mut current_node: Option<TermNode> = None;

//     for event in reader {
//         match event {
//             Ok(XmlEvent::StartElement { name, attributes, .. }) => {
//                 let term_node = TermNode::new(name.local_name, attributes, None);
//                 term_node.process();
//                 current_node = Some(term_node);
//             }
//             Ok(XmlEvent::EndElement { .. }) => {
//                 if let Some(node) = current_node.take() {
//                     process_xml_node(node);
//                 }
//             }
//             Ok(XmlEvent::Characters(data)) => {
//                 if let Some(node) = current_node.as_mut() {
//                     node.character_data = Some(data.trim().to_string());
//                 }
//             }
//             Err(e) => {
//                 println!("Error: {}", e);
//                 break;
//             }
//             _ => {}
//         }
//     }
// }
