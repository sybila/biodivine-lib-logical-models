use std::{fmt::Debug, io::BufRead, str::FromStr};
use thiserror::Error;
use xml::reader::XmlEvent;

use super::{expect_closure_of, expect_opening, expect_opening_of, XmlReader};

#[derive(Debug)]
pub enum Expression<T> {
    Terminal(Proposition<T>),
    Not(Box<Expression<T>>),
    // And(Box<Expression<T>>, Box<Expression<T>>),
    // Or(Box<Expression<T>>, Box<Expression<T>>),
    And(Vec<Expression<T>>), // cnf
    Or(Vec<Expression<T>>),  // dnf
    Xor(Box<Expression<T>>, Box<Expression<T>>),
    Implies(Box<Expression<T>>, Box<Expression<T>>),
}

impl<T: Ord + Clone> Expression<T> {
    pub fn highest_value_used_with_variable(&self, variable_name: &str) -> Option<T> {
        match self {
            Expression::Terminal(prop) => {
                if prop.ci == variable_name {
                    Some(prop.cn.clone())
                } else {
                    None
                }
            }
            Expression::Not(inner) => inner.highest_value_used_with_variable(variable_name),
            Expression::And(inner) => inner
                .iter()
                .filter_map(|expr| expr.highest_value_used_with_variable(variable_name))
                .max(),
            Expression::Or(inner) => inner
                .iter()
                .filter_map(|expr| expr.highest_value_used_with_variable(variable_name))
                .max(),
            Expression::Xor(lhs, rhs) => [lhs, rhs]
                .iter()
                .filter_map(|expr| expr.highest_value_used_with_variable(variable_name))
                .max(),
            Expression::Implies(lhs, rhs) => [lhs, rhs]
                .iter()
                .filter_map(|expr| expr.highest_value_used_with_variable(variable_name))
                .max(),
        }
    }
}

impl<T: Ord + Clone + Debug> Expression<T> {
    /// function works entirely the same as `highest_value_used_with_variable` but provides debug prints
    /// displaying all the propositions that compare the variable with higher values than its domain is
    pub fn highest_value_used_with_variable_detect_higher_than_exected(
        &self,
        variable_name: &str,
        expected_highest: T,
    ) -> Option<T> {
        match self {
            Expression::Terminal(prop) => {
                if prop.ci == variable_name {
                    let value_being_compared_to = prop.cn.clone();

                    if value_being_compared_to > expected_highest {
                        println!(
                            "[debug] variable {} only can take values in the [0; {:?}], but is being compared to with {:?} in proposition {:?}",
                            prop.ci,
                            expected_highest,
                            value_being_compared_to,
                            self
                        )
                    }

                    Some(prop.cn.clone())
                } else {
                    None
                }
            }
            // Expression::Not(inner) => inner.highest_value_used_with_variable(variable_name),
            Expression::Not(inner) => inner
                .highest_value_used_with_variable_detect_higher_than_exected(
                    variable_name,
                    expected_highest,
                ),
            Expression::And(inner) => inner
                .iter()
                .filter_map(|expr| {
                    expr.highest_value_used_with_variable_detect_higher_than_exected(
                        variable_name,
                        expected_highest.clone(),
                    )
                })
                .max(),
            Expression::Or(inner) => inner
                .iter()
                .filter_map(|expr| {
                    expr.highest_value_used_with_variable_detect_higher_than_exected(
                        variable_name,
                        expected_highest.clone(),
                    )
                })
                .max(),
            Expression::Xor(lhs, rhs) => [lhs, rhs]
                .iter()
                .filter_map(|expr| {
                    expr.highest_value_used_with_variable_detect_higher_than_exected(
                        variable_name,
                        expected_highest.clone(),
                    )
                })
                .max(),
            Expression::Implies(lhs, rhs) => [lhs, rhs]
                .iter()
                .filter_map(|expr| {
                    expr.highest_value_used_with_variable_detect_higher_than_exected(
                        variable_name,
                        expected_highest.clone(),
                    )
                })
                .max(),
        }
    }
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
        LogicOp::Xor => {
            expect_opening_of("apply", xml)?;
            let inner_lhs = Expression::try_from_xml(xml)?;
            expect_opening_of("apply", xml)?;
            let inner_rhs = Expression::try_from_xml(xml)?;
            expect_closure_of("apply", xml)?; // close *this* apply tag ie the one wrapping the op
            Ok(Expression::Xor(Box::new(inner_lhs), Box::new(inner_rhs)))
        }
        LogicOp::Implies => {
            expect_opening_of("apply", xml)?;
            let inner_lhs = Expression::try_from_xml(xml)?;
            expect_opening_of("apply", xml)?;
            let inner_rhs = Expression::try_from_xml(xml)?;
            expect_closure_of("apply", xml)?; // close *this* apply tag ie the one wrapping the op
            Ok(Expression::Implies(
                Box::new(inner_lhs),
                Box::new(inner_rhs),
            ))
        }
        LogicOp::And => {
            let cnf_items = get_cnf_or_dnf_items::<T, XR, BR>(xml)?;
            Ok(Expression::And(cnf_items))
        }
        LogicOp::Or => {
            let dnf_items = get_cnf_or_dnf_items::<T, XR, BR>(xml)?;
            Ok(Expression::Or(dnf_items))
        }
    }
}

// this could be probably done using process_list, but scuffed so better new fn
/// expects the xml reader to be set so that calling `next()` should encounter
/// either opening of `apply` (encountering the first element), or end of
/// `apply` (signaling the end of the cnf/dnf arguments (so empty cnf/dnf))
fn get_cnf_or_dnf_items<T: FromStr, XR: XmlReader<BR>, BR: BufRead>(
    xml: &mut XR,
) -> Result<Vec<Expression<T>>, Box<dyn std::error::Error>> {
    let mut acc = Vec::<Expression<T>>::new();

    loop {
        match xml.next() {
            Ok(XmlEvent::Whitespace(_)) => { /* ignore */ }
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "apply" {
                    acc.push(Expression::try_from_xml(xml)?);
                    continue;
                }

                return Err(format!(
                    "expected opening of indented apply or closing of this cnf/dnf apply, got opening of {}",
                    name.local_name
                )
                .into());
            }
            Ok(XmlEvent::EndElement { name, .. }) => {
                if name.local_name == "apply" {
                    return Ok(acc);
                }

                return Err(format!(
                    "expected opening of indented apply or closing of this cnf/dnf apply, got closing of {}",
                    name.local_name
                )
                .into());
            }
            Ok(XmlEvent::EndDocument) => {
                return Err("unexpected end of document".into());
            }
            other => {
                return Err(format!(
                    "expected either opening of indented apply or closing of this cnf/dnf apply, got {:?}",
                    other
                )
                .into());
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
                        content.trim()
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
                        content.trim()
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

// todo this should be impl FromStr & impl ToString
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
        // todo `TerminalOps` are specific for xml loading; this struct should not care about it
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
