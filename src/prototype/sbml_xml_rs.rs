use core::panic;
use std::io::BufRead;
use thiserror::Error;
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
pub enum CmpOp {
    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,
}

impl CmpOp {
    pub fn try_from_str(s: &str) -> Result<Self, ParseCmpOpError> {
        match s {
            "eq" => Ok(Self::Eq),
            "neq" => Ok(Self::Neq),
            "lt" => Ok(Self::Lt),
            "leq" => Ok(Self::Leq),
            "gt" => Ok(Self::Gt),
            "geq" => Ok(Self::Geq),
            _ => Err(ParseCmpOpError(s.to_string())),
        }
    }

    pub fn flip(&self) -> Self {
        match self {
            Self::Eq => Self::Eq,
            Self::Neq => Self::Neq,
            Self::Lt => Self::Gt,
            Self::Leq => Self::Geq,
            Self::Gt => Self::Lt,
            Self::Geq => Self::Leq,
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid comparison operator: {0}")]
pub struct ParseCmpOpError(String);

/// sbml proposition<br>
/// normalized ie in the form of `var op const`<br>
#[derive(Debug)]
pub struct Proposition {
    pub cmp: CmpOp,
    pub ci: String, // the variable name
    pub cn: u16,    // the constant value
}

#[derive(Debug, Error)]
#[error("Invalid proposition: {0}")]
pub struct ParsePropositionError(String);

pub fn parse_apply_element<T: BufRead>(
    xml: &mut EventReader<T>,
) -> Result<Proposition, Box<dyn std::error::Error>> {
    let mut ci: Option<String> = None; // the variable name
    let mut cn: Option<u16> = None; // the constant value
    let mut cmp: Option<CmpOp> = None; // comparison operator

    loop {
        match xml.next() {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "apply" => (), // todo not implemented
                // todo abstract this bs using builder once not prototype
                "ci" => {
                    if ci.is_some() {
                        return Err(Box::new(ParsePropositionError(
                            "duplicit ci element".to_string(),
                        )));
                    }

                    // todo move this to cn

                    if cmp.is_none() {
                        return Err(Box::new(ParsePropositionError(
                            "cmp op must be first".to_string(),
                        )));
                    }

                    let hopefully_ci_val = xml.next()?;
                    match hopefully_ci_val {
                        XmlEvent::Characters(s) => {
                            ci = Some(s);
                        }
                        _ => {
                            return Err(Box::new(ParsePropositionError(
                                "ci must be followed by characters - the variable name".to_string(),
                            )));
                        }
                    }
                }
                "cn" => {
                    if cn.is_some() {
                        return Err(Box::new(ParsePropositionError(
                            "duplicit cn element".to_string(),
                        )));
                    }

                    // todo should i care abt such bs?
                    if attributes
                        .iter()
                        .filter(|a| a.name.local_name == "type" && a.value == "integer")
                        .count()
                        != 1
                    {
                        return Err(Box::new(ParsePropositionError(
                            "ci must have exactly one attr of type=\"integer\" specified"
                                .to_string(),
                        )));
                    }

                    if cmp.is_none() {
                        return Err(Box::new(ParsePropositionError(
                            "cmp op must be first".to_string(),
                        )));
                    }

                    if ci.is_none() {
                        // input cn is lhs -> flip cmp to normalize
                        cmp = Some(cmp.unwrap().flip());
                    }

                    let hopefully_cn_val = xml.next()?;
                    match hopefully_cn_val {
                        XmlEvent::Characters(s) => {
                            cn = Some(s.parse::<u16>()?);
                        }
                        _ => {
                            return Err(Box::new(ParsePropositionError(
                                "cn must be followed by characters - ie contain int value"
                                    .to_string(),
                            )));
                        }
                    }
                }
                hopefully_cmp_op => {
                    let cmp_op = CmpOp::try_from_str(hopefully_cmp_op)?;
                    if cmp.is_some() {
                        return Err(Box::new(ParsePropositionError(
                            "duplicit cmp op".to_string(),
                        )));
                    }

                    cmp = Some(cmp_op);
                }
            },
            Ok(XmlEvent::EndElement { name, .. }) => {
                println!("ending {:?}", &name);
                if name.local_name == "apply" {
                    return Ok(Proposition {
                        cmp: cmp.unwrap(),
                        ci: ci.unwrap(),
                        cn: cn.unwrap(),
                    });
                }
            }
            Err(e) => panic!("Error: {}", e),
            _ => (),
        }
    }
}
