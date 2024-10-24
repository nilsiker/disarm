use std::collections::BTreeMap;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum ArmExpression {
    Literal(LiteralValue),
    Function(FunctionExpression),
    Parameter(String),
    Variable(String),
    Reference(ReferenceExpression),
    None,
}

fn parse_expression<E>(value: &str) -> Result<ArmExpression, E>
where
    E: de::Error,
{
    dbg!(value);
    match value {
        _ if value.is_empty() => Ok(ArmExpression::None),
        _ if value.starts_with("[variables(") => {
            let inner = &value[12..value.len() - 3];
            Ok(ArmExpression::Variable(inner.to_string()))
        }
        _ if value.starts_with("variables(") => {
            let inner = &value[11..value.len() - 2];
            Ok(ArmExpression::Variable(inner.to_string()))
        }
        _ if value.starts_with("[parameters(") => {
            let inner = &value[13..value.len() - 3];
            Ok(ArmExpression::Parameter(inner.to_string()))
        }
        _ if value.starts_with("parameters(") => {
            let inner = &value[12..value.len() - 2];
            Ok(ArmExpression::Parameter(inner.to_string()))
        }
        _ if value.starts_with("[") => {
            let first_opening_parenthesis = value.find("(").expect("opening parenthesis");
            let last_closing_parenthesis = value.rfind(")").expect("closing bracket");

            let function_name_str = &value[1..first_opening_parenthesis]; // Extract arguments from "format(...)"
            let args_str = &value[first_opening_parenthesis + 1..last_closing_parenthesis];
            let args: Vec<&str> = args_str.split(",").map(|s| s.trim()).collect(); // Split and trim arguments

            // For simplicity, assuming arguments are either strings or other simple literals
            let parsed_args: Vec<ArmExpression> = args
                .iter()
                .map(|arg| parse_expression::<E>(arg).expect("parseable"))
                .collect();

            let function_name = match function_name_str {
                "format" => FunctionName::Format,
                "concat" => FunctionName::Concat,
                "copyIndex" => FunctionName::CopyIndex,
                "resourceId" => FunctionName::ResourceId,
                "if" => FunctionName::If,
                "resourceGroup" => FunctionName::ResourceGroup,
                _ => todo!(),
            };

            return Ok(ArmExpression::Function(FunctionExpression {
                name: function_name,
                arguments: parsed_args,
            }));
        }
        // TODO messy argument check, find a better way to parse functions/args?
        _ if value.starts_with("'") => Ok(ArmExpression::Literal(LiteralValue::String(
            value[1..value.len() - 1].to_string(),
        ))),
        // TODO We are still in arguments, and could find a function!
        _ => Ok(ArmExpression::Literal(LiteralValue::String(
            value.to_string(),
        ))),
    }
}
impl<'de> Deserialize<'de> for ArmExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArmExpressionVisitor;

        impl<'de> Visitor<'de> for ArmExpressionVisitor {
            type Value = ArmExpression;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid ARM expression string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_expression(value)
            }
        }

        // Deserialize the string by using our visitor
        deserializer.deserialize_str(ArmExpressionVisitor)
    }
}

// Represents literal values like strings, numbers, and booleans
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FunctionExpression {
    pub name: FunctionName,
    pub arguments: Vec<ArmExpression>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ReferenceExpression {
    pub resource_name: String,
    pub api_version: Option<String>, // Some references may require an API version
}

// Example predefined functions like concat(), resourceId(), etc.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum FunctionName {
    Concat,
    ResourceId,
    CopyIndex,
    Format,
    If,
    ResourceGroup,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArmTemplate {
    pub parameters: Option<BTreeMap<String, ArmParameter>>,
    pub variables: Option<BTreeMap<String, ArmExpression>>,
    pub resources: Vec<ArmResource>,
    pub outputs: Option<Vec<ArmOutput>>,
}

// ARM template parameters
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArmParameter {
    pub r#type: String, // e.g., string, int, bool
    #[serde(rename = "defaultValue")]
    pub default_value: Option<ArmExpression>,
}

// ARM template resources
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArmResource {
    pub name: ArmExpression,
    pub r#type: String, // e.g., Microsoft.Compute/virtualMachines
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub depends_on: Option<Vec<Box<ArmResource>>>,
}

// ARM template outputs
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArmOutput {
    pub name: String,
    pub value: ArmExpression,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let key = ArmTemplate {
            parameters: Some(BTreeMap::from([(
                "functionAppName".to_string(),
                ArmParameter {
                    r#type: "string".to_string(),
                    default_value: None,
                },
            )])),
            variables: Some(BTreeMap::from([])),
            resources: vec![],
            outputs: None,
        };
        let file = std::fs::File::open("data/function-app-dedicated-plan.json").expect("exists");
        let template: ArmTemplate = serde_json::from_reader(file).expect("parseable");

        pretty_assertions::assert_eq!(template, key);
    }
}
