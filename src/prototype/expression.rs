use std::{io::BufRead, str::FromStr};
use thiserror::Error;
use xml::reader::XmlEvent;

use crate::{expect_closure_of, expect_opening, expect_opening_of, XmlReader};

#[derive(Debug)]
pub enum Expression<T> {
    Terminal(Proposition<T>),
    Not(Box<Expression<T>>),
    And(Box<Expression<T>>, Box<Expression<T>>),
    Or(Box<Expression<T>>, Box<Expression<T>>),
    Xor(Box<Expression<T>>, Box<Expression<T>>),
    Implies(Box<Expression<T>>, Box<Expression<T>>),
}

enum LogicOp {
    Not,
    And,
    Or,
    Xor,
    Implies,
}

impl LogicOp {
    fn from_str(op: &str) -> Option<Self> {
        match op {
            "not" => Some(Self::Not),
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "xor" => Some(Self::Xor),
            "implies" => Some(Self::Implies),
            _ => None,
        }
    }

    fn its_string(&self) -> String {
        match self {
            Self::Not => "not".to_string(),
            Self::And => "and".to_string(),
            Self::Or => "or".to_string(),
            Self::Xor => "xor".to_string(),
            Self::Implies => "implies".to_string(),
        }
    }
}

// todo there should be constraint on the type of T: FromStr
impl<T: FromStr> Expression<T> {
    // todo consider iterative approach instead of recursive?
    pub fn try_from_xml<XR: XmlReader<BR>, BR: BufRead>(
        xml: &mut XR,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    let op = name.local_name.as_str();
                    if let Some(log_op) = LogicOp::from_str(op) {
                        return logical_from_xml(log_op, xml);
                    }

                    if let Ok(cmp_op) = CmpOp::try_from_str(op) {
                        return terminal_from_xml(cmp_op, xml);
                    }

                    // lol rust-analyzer kinda choked on the next line
                    return Err(format!("expected one of not, and, or, xor, implies, eq, neq, lt, leq, gt, geq, found {}", op).into());
                }
                Ok(xml::reader::XmlEvent::EndElement { name, .. }) => {
                    return Err(format!("unexpected end of element {}", name.local_name).into())
                }
                Ok(xml::reader::XmlEvent::EndDocument) => {
                    return Err("unexpected end of document".into())
                }
                Err(e) => panic!("Error: {}", e),
                _ => (),
            }
        }
    }
}

fn terminal_from_xml<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    op: CmpOp,
    xml: &mut XR,
) -> Result<Expression<T>, Box<dyn std::error::Error>> {
    expect_closure_of(&op.its_string(), xml)?; // close the cmp op tag
    let prp = parse_terminal_ops(xml)?;
    expect_closure_of("apply", xml)?;
    Ok(Expression::Terminal(Proposition::new(op, prp)))
}

fn logical_from_xml<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    op: LogicOp,
    xml: &mut XR,
) -> Result<Expression<T>, Box<dyn std::error::Error>> {
    // expect_closure_of(&op.its_string(), xml)?; // self closing tag must be "closed"
    expect_closure_of(&op.its_string(), xml)?;
    match op {
        LogicOp::Not => {
            expect_opening_of("apply", xml)?;
            let inner = Expression::try_from_xml(xml)?;
            expect_closure_of("apply", xml)?; // close *this* apply tag ie the one wrapping the op
            Ok(Expression::Not(Box::new(inner)))
        }
        LogicOp::And | LogicOp::Or | LogicOp::Xor | LogicOp::Implies => {
            expect_opening_of("apply", xml)?;
            let inner_lhs = Expression::try_from_xml(xml)?;
            expect_opening_of("apply", xml)?;
            let inner_rhs = Expression::try_from_xml(xml)?;
            expect_closure_of("apply", xml)?; // close *this* apply tag ie the one wrapping the op
            match op {
                LogicOp::And => Ok(Expression::And(Box::new(inner_lhs), Box::new(inner_rhs))),
                LogicOp::Or => Ok(Expression::Or(Box::new(inner_lhs), Box::new(inner_rhs))),
                LogicOp::Xor => Ok(Expression::Xor(Box::new(inner_lhs), Box::new(inner_rhs))),
                LogicOp::Implies => Ok(Expression::Implies(
                    Box::new(inner_lhs),
                    Box::new(inner_rhs),
                )),
                _ => unreachable!(), // because of the not
            }
        }
    }
}

pub enum TerminalOps<T> {
    Standard(String, T),
    Flipped(T, String),
}

/// expects input xml to be in such state that the next two xml nodes are either
/// ci and then cn, or cn and then ci
pub fn parse_terminal_ops<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
) -> Result<TerminalOps<T>, Box<dyn std::error::Error>> {
    let elem = expect_opening(xml)?.name.local_name;
    if elem == "ci" {
        let ci;

        if let XmlEvent::Characters(content) = xml.next()? {
            // ci = Some(content.parse::<u16>()?);
            ci = content.trim().to_string();
        } else {
            return Err("ci must be followed by characters - the variable name".into());
        }
        expect_closure_of("ci", xml)?;

        expect_opening_of("cn", xml)?;
        let cn;
        if let XmlEvent::Characters(content) = xml.next()? {
            cn = match content.trim().parse::<T>() {
                Ok(it) => it,
                Err(_) => {
                    return Err(format!(
                        "could not parse to specified type; got {}",
                        content.trim().to_string()
                    )
                    .into())
                }
            }
        } else {
            return Err("cn must be followed by characters - the variable name".into());
        }
        expect_closure_of("cn", xml)?;

        return Ok(TerminalOps::Standard(ci, cn));
    }

    if elem == "cn" {
        let cn;
        if let XmlEvent::Characters(content) = xml.next()? {
            cn = match content.trim().parse::<T>() {
                Ok(it) => it,
                Err(_) => {
                    return Err(format!(
                        "could not parse to specified type; got {}",
                        content.trim().to_string()
                    )
                    .into())
                }
            }
        } else {
            return Err("cn must be followed by characters - the variable name".into());
        }
        expect_closure_of("cn", xml)?;

        expect_opening_of("ci", xml)?;
        let ci;
        if let XmlEvent::Characters(content) = xml.next()? {
            ci = content;
        } else {
            return Err("ci must be followed by characters - the variable name".into());
        }
        expect_closure_of("ci", xml)?;

        return Ok(TerminalOps::Flipped(cn, ci));
    }

    Err("expected ci or cn".into())
}

#[derive(Debug, Clone)]
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

    pub fn its_string(&self) -> String {
        match self {
            Self::Eq => "eq".to_string(),
            Self::Neq => "neq".to_string(),
            Self::Lt => "lt".to_string(),
            Self::Leq => "leq".to_string(),
            Self::Gt => "gt".to_string(),
            Self::Geq => "geq".to_string(),
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

/// sbml proposition normalized ie in the form of `var op const`
#[derive(Clone, Debug)]
pub struct Proposition<T> {
    pub cmp: CmpOp,
    pub ci: String, // the variable name
    pub cn: T,      // the constant value
}

impl<T> Proposition<T> {
    /// if the input terminal operands are flipped, the returning value will flip the operands as well
    /// as the comparison operator in order to normalize the proposition
    pub fn new(cmp_op: CmpOp, ops: TerminalOps<T>) -> Self {
        match ops {
            TerminalOps::Standard(lhs, rhs) => Self {
                cmp: cmp_op,
                ci: lhs,
                cn: rhs,
            },
            TerminalOps::Flipped(lhs, rhs) => Self {
                cmp: cmp_op.flip(),
                ci: rhs,
                cn: lhs,
            },
        }
    }
}
