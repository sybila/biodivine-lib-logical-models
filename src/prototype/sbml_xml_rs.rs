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

struct PropositionBuilder {
    _cmp: Option<CmpOp>,
    _ci: Option<String>,
    _cn: Option<u16>,
}

impl PropositionBuilder {
    pub fn new() -> Self {
        Self {
            _cmp: None,
            _ci: None,
            _cn: None,
        }
    }

    pub fn cmp(&mut self, cmp: &str) -> Result<&mut Self, ParsePropositionError> {
        if self._cmp.is_some() {
            return Err(ParsePropositionError("duplicit cmp element".to_string()));
        }

        self._cmp =
            Some(CmpOp::try_from_str(cmp).map_err(|e| ParsePropositionError(e.to_string()))?);

        Ok(self)
    }

    pub fn ci(&mut self, ci: XmlEvent) -> Result<&mut Self, ParsePropositionError> {
        if self._ci.is_some() {
            return Err(ParsePropositionError("duplicit ci element".to_string()));
        }

        if self._cmp.is_none() {
            return Err(ParsePropositionError(
                // enforce order, because of its semantics (lhs vs rhs)
                "cmp op must be set before ci".to_string(),
            ));
        }

        match ci {
            XmlEvent::Characters(s) => {
                // todo should i validate it somehow? or eg truncate whitespaces?
                self._ci = Some(s);
            }
            _ => {
                return Err(ParsePropositionError(
                    "ci must be followed by characters - the variable name".to_string(),
                ));
            }
        }

        Ok(self)
    }

    pub fn cn(&mut self, cn: XmlEvent) -> Result<&mut Self, ParsePropositionError> {
        if self._cn.is_some() {
            return Err(ParsePropositionError("duplicit cn element".to_string()));
        }

        if self._cmp.is_none() {
            return Err(ParsePropositionError(
                // enforce order, because of its semantics (lhs vs rhs)
                "cmp op must be set before cn".to_string(),
            ));
        }

        // todo should check attributes? i mean checked implicitly with the following code

        match cn {
            XmlEvent::Characters(s) => {
                // todo should i validate it somehow? or eg truncate whitespaces?
                let ok_val = s.parse::<u16>().map_err(|_| {
                    ParsePropositionError(format!("could not parse cn value {s} to u16"))
                })?;
                self._cn = Some(ok_val);
            }
            _ => {
                return Err(ParsePropositionError(
                    "cn must be followed by characters - the constant value".to_string(),
                ));
            }
        }

        if self._ci.is_none() {
            self._cmp.as_mut().unwrap().flip(); // safe unwrap; checkd above
        }

        Ok(self)
    }

    // consumes the builder; think this makes sense
    pub fn build(self) -> Result<Proposition, ParsePropositionError> {
        Ok(Proposition {
            cmp: self
                ._cmp
                .ok_or(ParsePropositionError("cmp op not set".to_string()))?,
            ci: self
                ._ci
                .ok_or(ParsePropositionError("ci not set".to_string()))?,
            cn: self
                ._cn
                .ok_or(ParsePropositionError("cn not set".to_string()))?,
        })
    }
}

#[derive(Debug, Error)]
#[error("Invalid proposition: {0}")]
pub struct ParsePropositionError(String);

pub fn parse_apply_element<T: BufRead>(
    xml: &mut EventReader<T>,
    // ) -> Result<Proposition, Box<dyn std::error::Error>> {
) -> Result<Proposition, ParsePropositionError> {
    let mut builder = PropositionBuilder::new();

    loop {
        match xml.next() {
            Ok(XmlEvent::StartElement { name, .. }) => match name.local_name.as_str() {
                "apply" => (), // todo not implemented
                "ci" => {
                    builder.ci(xml.next().map_err(|_| {
                        ParsePropositionError("underlying xml reader failed".to_string())
                    })?)?;
                }
                "cn" => {
                    // builder.cn(xml.next()?)?;
                    builder.cn(xml.next().map_err(|_| {
                        ParsePropositionError("underlying xml reader failed".to_string())
                    })?)?;
                }
                n if CmpOp::try_from_str(n).is_ok() => {
                    builder.cmp(n)?;
                }
                _ => (),
            },
            Ok(XmlEvent::EndElement { name, .. }) => {
                if name.local_name == "apply" {
                    return builder.build();
                }
            }
            Ok(XmlEvent::EndDocument) => {
                return Err(ParsePropositionError(
                    "unexpected end of document".to_string(),
                ));
            }
            Err(e) => panic!("Error: {}", e),
            _ => (),
        }
    }
}
