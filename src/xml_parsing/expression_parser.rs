#![allow(dead_code)] // todo remove

use std::{io::BufRead, str::FromStr};

use xml::reader::XmlEvent;

use crate::{
    expression_components::{
        expression::Expression,
        proposition::{ComparisonOperator, Proposition},
    },
    xml_parsing::utils::{expect_closure_of, expect_opening},
};

use super::{
    utils::{expect_opening_of, XmlReadingError},
    xml_reader::XmlReader,
};

enum LogicalOperator {
    Not,
    And,
    Or,
    Xor,
    Implies,
}

impl FromStr for LogicalOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not" => Ok(Self::Not),
            "and" => Ok(Self::And),
            "or" => Ok(Self::Or),
            "xor" => Ok(Self::Xor),
            "implies" => Ok(Self::Implies),
            _ => Err(()),
        }
    }
}

impl ToString for LogicalOperator {
    fn to_string(&self) -> String {
        match self {
            Self::Not => "not",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Implies => "implies",
        }
        .to_string()
    }
}

impl<T: FromStr> Expression<T> {
    pub fn try_from_xml<XR, BR>(xml: &mut XR) -> Result<Self, XmlReadingError>
    where
        XR: XmlReader<BR>,
        BR: BufRead,
    {
        loop {
            match xml.next()? {
                XmlEvent::Whitespace(_) => ( /* ignore */ ),
                XmlEvent::StartElement { name, .. } => {
                    let received_operator = name.local_name.as_str();

                    if let Ok(received_logical_operator) =
                        received_operator.parse::<LogicalOperator>()
                    {
                        return logical_from_xml(xml, received_logical_operator);
                    }

                    if let Ok(comparison_operator) = received_operator.parse::<ComparisonOperator>()
                    {
                        return Ok(Expression::Terminal(proposition_from_xml(
                            xml,
                            comparison_operator,
                        )?));
                    }
                }
                other => {
                    return Err(XmlReadingError::UnexpectedEvent {
                        expected: super::utils::ExpectedXmlEvent::Start(
                            "any logical operator or comparison operator".to_string(),
                        ),
                        got: other,
                    })
                }
            }
        }
    }
}

fn logical_from_xml<XR, BR, T>(
    xml: &mut XR,
    logical_operator: LogicalOperator,
) -> Result<Expression<T>, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
    T: FromStr,
{
    expect_closure_of(xml, &logical_operator.to_string())?; // "close" the self-closing tag

    match logical_operator {
        LogicalOperator::Not => {
            expect_opening_of(xml, "apply")?; // "open" the inner apply tag
            let inner_expression = Expression::try_from_xml(xml)?;
            expect_closure_of(xml, "apply")?; // "close" the *this* apply tag
            Ok(inner_expression)
        }
        LogicalOperator::And => unimplemented!(),
        LogicalOperator::Or => unimplemented!(),
        LogicalOperator::Xor => {
            expect_opening_of(xml, "apply")?; // "open" the first inner apply tag
            let lhs = Expression::try_from_xml(xml)?;
            expect_opening_of(xml, "apply")?; // "open" the second inner apply tag
            let rhs = Expression::try_from_xml(xml)?;
            expect_closure_of(xml, "apply")?; // "close" the *this* apply tag
            Ok(Expression::Xor(Box::new(lhs), Box::new(rhs)))
        }
        LogicalOperator::Implies => {
            expect_opening_of(xml, "apply")?; // "open" the first inner apply tag
            let lhs = Expression::try_from_xml(xml)?;
            expect_opening_of(xml, "apply")?; // "open" the second inner apply tag
            let rhs = Expression::try_from_xml(xml)?;
            expect_closure_of(xml, "apply")?; // "close" the *this* apply tag
            Ok(Expression::Implies(Box::new(lhs), Box::new(rhs)))
        }
    }
}

// todo maybe try to rewrite this using the `prcess_list` function
fn get_clausule_items<XR, BR, T>(xml: &mut XR) -> Result<Vec<Expression<T>>, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
    T: FromStr,
{
    let mut acc = Vec::<Expression<T>>::new();

    loop {
        match xml.next()? {
            XmlEvent::Whitespace(_) => ( /* ignore */ ),
            ref got @ XmlEvent::StartElement { ref name, .. } => {
                if name.local_name == "apply" {
                    acc.push(Expression::try_from_xml(xml)?);
                    continue;
                }

                return Err(XmlReadingError::UnexpectedEvent {
                    expected: super::utils::ExpectedXmlEvent::Start("apply".to_string()),
                    got: got.clone(),
                });
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == "apply" {
                    return Ok(acc);
                }

                return Err(XmlReadingError::UnexpectedEvent {
                    expected: super::utils::ExpectedXmlEvent::End("apply".to_string()),
                    got: XmlEvent::EndElement { name },
                });
            }
            other => {
                return Err(XmlReadingError::UnexpectedEvent {
                    expected: super::utils::ExpectedXmlEvent::Start("start of ".to_string()),
                    got: other,
                })
            }
        }
    }
}

fn proposition_from_xml<XR, BR, T>(
    xml: &mut XR,
    comparison_operator: ComparisonOperator,
) -> Result<Proposition<T>, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
    T: FromStr,
{
    let element = expect_opening(xml)?;

    match element.name.local_name.as_str() {
        "ci" => {
            let variable_name = get_variable_name(xml)?;

            expect_opening_of(xml, "cn")?;
            let constant_value = get_constant_value(xml)?;

            Ok(Proposition::new(
                comparison_operator,
                variable_name,
                constant_value,
            ))
        }
        "cn" => {
            let constant_value = get_constant_value(xml)?;

            expect_opening_of(xml, "ci")?;
            let variable_name = get_variable_name(xml)?;

            Ok(Proposition::new(
                comparison_operator,
                variable_name,
                constant_value,
            ))
        }
        _ => Err(XmlReadingError::UnexpectedEvent {
            expected: super::utils::ExpectedXmlEvent::Start("ci or cn".to_string()),
            got: XmlEvent::StartElement {
                name: element.name,
                attributes: element.attributes,
                namespace: element.namespace,
            },
        }),
    }
}

fn get_variable_name<XR, BR>(xml: &mut XR) -> Result<String, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
{
    let variable_name = match xml.next()? {
        XmlEvent::Characters(variable_name) => variable_name,
        other => {
            return Err(XmlReadingError::UnexpectedEvent {
                expected: super::utils::ExpectedXmlEvent::Characters,
                got: other,
            })
        }
    };

    expect_closure_of(xml, "ci")?;

    Ok(variable_name)
}

fn get_constant_value<XR, BR, T>(xml: &mut XR) -> Result<T, XmlReadingError>
where
    XR: XmlReader<BR>,
    BR: BufRead,
    T: FromStr,
{
    let constant_value = match xml.next()? {
        XmlEvent::Characters(constant_value) => constant_value
            .parse::<T>()
            .map_err(|_| XmlReadingError::ParsingError(constant_value))?,
        other => {
            return Err(XmlReadingError::UnexpectedEvent {
                expected: super::utils::ExpectedXmlEvent::Characters,
                got: other,
            })
        }
    };

    expect_closure_of(xml, "cn")?;

    Ok(constant_value)
}
