use std::io::BufRead;
use thiserror::Error;
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
pub enum Expression {
    Terminal(Proposition),
    Not(Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Xor(Box<Expression>, Box<Expression>),
    Implies(Box<Expression>, Box<Expression>),
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

impl Expression {
    // todo consider iterative approach instead of recursive?
    pub fn try_from_xml<T: BufRead>(
        xml: &mut EventReader<T>,
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

                    return Err(format!("expected one of {{ not, and, or, xor, implies, eq, neq, lt, leq, gt, geq }}, found {}",op).into());
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

fn terminal_from_xml(
    op: CmpOp,
    xml: &mut EventReader<impl BufRead>,
) -> Result<Expression, Box<dyn std::error::Error>> {
    expect_closure_of(&op.its_string(), xml)?; // close the cmp op tag
    let prp = parse_terminal_ops(xml)?;
    expect_closure_of("apply", xml)?;
    Ok(Expression::Terminal(Proposition::new(op, prp)))
}

fn logical_from_xml<T: BufRead>(
    op: LogicOp,
    xml: &mut EventReader<T>,
) -> Result<Expression, Box<dyn std::error::Error>> {
    expect_closure_of(&op.its_string(), xml)?; // self closing tag must be "closed"
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

fn expect_opening_of<T: BufRead>(
    expected: &str,
    xml: &mut EventReader<T>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* whitespace is the reason we want to loop */ }
            Ok(XmlEvent::StartElement { name, .. }) => {
                return if name.local_name == expected {
                    Ok(())
                } else {
                    Err(format!(
                        "expected opening element {}, got {}",
                        expected, name.local_name
                    )
                    .into())
                }
            }
            any => return Err(format!("expected opening of {}, got {:?}", expected, any).into()),
        }
    }
}

fn expect_closure_of<T: BufRead>(
    expected: &str,
    xml: &mut EventReader<T>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* whitespace is the reason we want to loop */ }
            Ok(XmlEvent::EndElement { name, .. }) => {
                return if name.local_name == expected {
                    Ok(())
                } else {
                    Err(format!("expected closing of {}, got {}", expected, name.local_name).into())
                }
            }
            any => return Err(format!("expected closing of {}, got {:?}", expected, any).into()),
        }
    }
}

pub enum TerminalOps {
    Standard(String, u16),
    Flipped(u16, String),
}

/// expects input xml to be in such state that the next two xml nodes are either
/// ci and then cn, or cn and then ci
pub fn parse_terminal_ops<T: BufRead>(
    xml: &mut EventReader<T>,
) -> Result<TerminalOps, Box<dyn std::error::Error>> {
    let elem = expect_opening(xml)?;
    if elem == "ci" {
        let ci;

        if let XmlEvent::Characters(content) = xml.next()? {
            // ci = Some(content.parse::<u16>()?);
            ci = content;
        } else {
            return Err("ci must be followed by characters - the variable name".into());
        }
        expect_closure_of("ci", xml)?;

        expect_opening_of("cn", xml)?;
        let cn;
        if let XmlEvent::Characters(content) = xml.next()? {
            cn = content.trim().parse::<u16>()?;
        } else {
            return Err("cn must be followed by characters - the variable name".into());
        }
        expect_closure_of("cn", xml)?;

        return Ok(TerminalOps::Standard(ci, cn));
    }

    if elem == "cn" {
        let cn;
        if let XmlEvent::Characters(content) = xml.next()? {
            cn = content.trim().parse::<u16>()?;
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

/// get the name of the opening tag
/// if the next event is not a start element, returns an error
/// if the next event is a start element, returns its name
fn expect_opening<T: BufRead>(
    xml: &mut EventReader<T>,
) -> Result<String, Box<dyn std::error::Error>> {
    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* whitespace is the reason we want to loop */ }
            Ok(XmlEvent::StartElement { name, .. }) => return Ok(name.local_name),
            any => return Err(format!("expected an opening, got {:?}", any).into()),
        }
    }
}

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
#[derive(Debug)]
pub struct Proposition {
    pub cmp: CmpOp,
    pub ci: String, // the variable name
    pub cn: u16,    // the constant value
}

impl Proposition {
    /// if the input terminal operands are flipped, the returning value will flip the operands as well
    /// as the comparison operator in order to normalize the proposition
    pub fn new(cmp_op: CmpOp, ops: TerminalOps) -> Self {
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
