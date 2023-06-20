use std::io::BufRead;
use thiserror::Error;
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
pub enum Expression {
    Terminal(Proposition),
    // Internal(Box<Expression>),
    Not(Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Xor(Box<Expression>, Box<Expression>),
    Implies(Box<Expression>, Box<Expression>),
}

impl Expression {
    pub fn dflt() -> Self {
        unimplemented!("default expression")
    }

    // todo consider iterative approach instead of recursive?
    pub fn try_from_xml<T: BufRead>(
        xml: &mut EventReader<T>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        loop {
            match xml.next() {
                Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                    match name.local_name.as_str() {
                        "not" => {
                            expect_closure_of(&name.local_name, xml)?; // self closing tag must be "closed"
                            expect_opening_of("apply", xml)?;
                            let inner = Expression::try_from_xml(xml)?;
                            expect_closure_of("apply", xml)?; // close *this* apply tag ie the one wrapping the <not/>
                            return Ok(Expression::Not(Box::new(inner)));
                        }
                        "and" => {
                            expect_closure_of(&name.local_name, xml)?; // self closing tag must be "closed"
                            expect_opening_of("apply", xml)?;
                            let inner_lhs = Expression::try_from_xml(xml)?;
                            expect_opening_of("apply", xml)?;
                            let inner_rhs = Expression::try_from_xml(xml)?;
                            expect_closure_of("apply", xml)?;
                            return Ok(Expression::And(Box::new(inner_lhs), Box::new(inner_rhs)));
                        }
                        "or" => {
                            expect_closure_of(&name.local_name, xml)?; // self closing tag must be "closed"
                            expect_opening_of("apply", xml)?;
                            let inner_lhs = Expression::try_from_xml(xml)?;
                            expect_opening_of("apply", xml)?;
                            let inner_rhs = Expression::try_from_xml(xml)?;
                            expect_closure_of("apply", xml)?;
                            return Ok(Expression::Or(Box::new(inner_lhs), Box::new(inner_rhs)));
                        }
                        "xor" => {
                            expect_closure_of(&name.local_name, xml)?; // self closing tag must be "closed"
                            expect_opening_of("apply", xml)?;
                            let inner_lhs = Expression::try_from_xml(xml)?;
                            expect_opening_of("apply", xml)?;
                            let inner_rhs = Expression::try_from_xml(xml)?;
                            expect_closure_of("apply", xml)?;
                            return Ok(Expression::Xor(Box::new(inner_lhs), Box::new(inner_rhs)));
                        }
                        "implies" => {
                            expect_closure_of(&name.local_name, xml)?; // self closing tag must be "closed"
                            expect_opening_of("apply", xml)?;
                            let inner_lhs = Expression::try_from_xml(xml)?;
                            expect_opening_of("apply", xml)?;
                            let inner_rhs = Expression::try_from_xml(xml)?;
                            expect_closure_of("apply", xml)?;
                            return Ok(Expression::Implies(
                                Box::new(inner_lhs),
                                Box::new(inner_rhs),
                            ));
                        }
                        // must_be_cmp_op => {
                        //     // todo oh fck already consumed the operator; need to pass it to parse_apply_element
                        //     let proposition = super::parse_apply_element(xml)?; // pass the op somehow
                        //     drain(xml, "apply")?; // clean the xml iterator; will be used
                        //     return Ok(Expression::Terminal(proposition));
                        // }
                        // "eq" => unimplemented!(),
                        // "neq" => unimplemented!(),
                        // "lt" => unimplemented!(),
                        // "leq" => unimplemented!(),
                        // "gt" => unimplemented!(),
                        // "geq" => unimplemented!(),
                        "eq" | "neq" | "lt" | "leq" | "gt" | "geq" => {
                            expect_closure_of(&name.local_name, xml)?; // close the cmp op tag
                            let prp = parse_terminal_ops(xml)?;
                            expect_closure_of("apply", xml)?; // todo likely not; alrdy inside parse_terminal_ops
                            return Ok(Expression::Terminal(Proposition::new(
                                CmpOp::try_from_str(&name.local_name)?,
                                prp,
                            )));
                        }
                        _ => {
                            return Err(format!(
                                "expected one of {{ not, and, or, xor, implies, eq, neq, lt, leq, gt, geq }}, found {}",
                                name.local_name
                            )
                            .into());
                        }
                    }
                }
                Ok(xml::reader::XmlEvent::EndElement { name, .. }) => {
                    if name.local_name == "apply" {
                        return Ok(Expression::dflt()); // todo not default ofc; build
                    }
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

// i imagine this is what the <not/> node looks like
// <apply>
//     <not/>
//     <apply>
//         ...
//     </apply>
// </apply>

fn expect_opening_of<T: BufRead>(
    expected: &str,
    xml: &mut EventReader<T>,
) -> Result<(), Box<dyn std::error::Error>> {
    print!("expecting opening {}...", expected);
    // match xml.next() {
    //     Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
    //         if name.local_name == expected {
    //             Ok(())
    //         } else {
    //             Err(format!(
    //                 "expected start element {} but found {}",
    //                 expected, name.local_name
    //             )
    //             .into())
    //         }
    //     }
    //     Ok(xml::reader::XmlEvent::EndElement { name, .. }) => Err(format!(
    //         "expected start element {} but found closing {}",
    //         expected, name.local_name
    //     )
    //     .into()),
    //     Ok(xml::reader::XmlEvent::EndDocument) => Err("unexpected end of document".into()),
    //     // Ok(unexp) => Err(format!(
    //     //     "unexpected event: {:?}; expected opening {} instead",
    //     //     unexp, expected
    //     // )
    //     // .into()),
    //     Ok(_) => {}
    //     Err(e) => Err(e.into()),
    // }
    loop {
        match xml.next() {
            Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                if name.local_name == expected {
                    println!("ok");
                    return Ok(());
                } else {
                    println!(
                        "expected start element {} but found {}",
                        expected, name.local_name
                    );
                    return Err(format!(
                        "expected start element {} but found {}",
                        expected, name.local_name
                    )
                    .into());
                }
            }
            Ok(xml::reader::XmlEvent::EndElement { name, .. }) => {
                println!(
                    "expected start element {} but found closing {}",
                    expected, name.local_name
                );
                return Err(format!(
                    "expected start element {} but found closing {}",
                    expected, name.local_name
                )
                .into());
            }
            Ok(xml::reader::XmlEvent::EndDocument) => {
                println!("unexpected end of document");
                return Err("unexpected end of document".into());
            }
            Ok(_) => {}
            Err(e) => {
                println!(
                    "unexpected event: {:?}; expected opening {} instead",
                    e, expected
                );
                return Err(e.into());
            }
        }
    }
}

fn expect_closure_of<T: BufRead>(
    expected: &str,
    xml: &mut EventReader<T>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("expecting closing {}...", expected);
    loop {
        match xml.next() {
            Ok(xml::reader::XmlEvent::StartElement { name, .. }) => {
                return Err(format!(
                    "expected closing element {} but found opening {}",
                    expected, name.local_name
                )
                .into())
            }
            Ok(xml::reader::XmlEvent::EndElement { name, .. }) => {
                if name.local_name == expected {
                    return Ok(());
                } else {
                    return Err(format!(
                        "expected closing element {} but found {}",
                        expected, name.local_name
                    )
                    .into());
                }
            }
            Ok(xml::reader::XmlEvent::EndDocument) => {
                return Err("unexpected end of document".into())
            }
            Ok(_) => { /* want to loop in this case */ } // return Err(format!("unexpected event: {:?}", unexp).into()),
            Err(e) => return Err(e.into()),
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
            Ok(XmlEvent::StartElement { name, .. }) => return Ok(name.local_name),
            Ok(XmlEvent::EndDocument) => return Err("unexpected end of document".into()),
            // Ok(unexp) => Err(format!("expected event {:?}", unexp).into()),
            Ok(_) => {
                // do not loop; there are some cases you want to skip like whitespaces...
                // return Err(format!("unexpected event: {:?}; expected any opening", unexp).into())
            }
            Err(_) => return Err("underlying xml reader failed".into()),
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

impl Proposition {
    pub fn new(cmp_op: CmpOp, ops: TerminalOps) -> Self {
        // if let TerminalOps::Standard(lhs, rhs) = ops {

        // }

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
