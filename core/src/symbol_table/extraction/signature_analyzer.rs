//! Function signature analysis and extraction

use anyhow::Result;
use tree_sitter::Node;

/// Analyzer for function and method signatures
pub struct SignatureAnalyzer;

impl SignatureAnalyzer {
    /// Extract function signature components
    pub fn analyze_function_signature(node: &Node, code: &str) -> Result<FunctionSignature> {
        let name = Self::extract_function_name(node, code)?;
        let parameters = Self::extract_parameters(node, code)?;
        let return_type = Self::extract_return_type(node, code)?;
        let visibility = Self::extract_visibility(node, code)?;
        let is_async = Self::is_async_function(node, code)?;
        let is_generic = Self::has_generic_parameters(node, code)?;

        Ok(FunctionSignature {
            name,
            parameters,
            return_type,
            visibility,
            is_async,
            is_generic,
        })
    }

    /// Extract function name
    fn extract_function_name(node: &Node, code: &str) -> Result<String> {
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" | "field_identifier" => {
                    let start = child.start_byte();
                    let end = child.end_byte();
                    return Ok(code[start..end].to_string());
                }
                _ => {}
            }
        }
        Err(anyhow::anyhow!("Could not extract function name"))
    }

    /// Extract function parameters
    fn extract_parameters(node: &Node, code: &str) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        for child in node.children(&mut node.walk()) {
            if child.kind() == "parameters" || child.kind() == "parameter_list" {
                for param_node in child.children(&mut child.walk()) {
                    if param_node.kind() == "parameter"
                        || param_node.kind() == "parameter_declaration"
                    {
                        if let Some(param) = Self::extract_parameter(&param_node, code)? {
                            parameters.push(param);
                        }
                    }
                }
                break;
            }
        }

        Ok(parameters)
    }

    /// Extract single parameter
    fn extract_parameter(node: &Node, code: &str) -> Result<Option<Parameter>> {
        let mut name = None;
        let mut param_type = None;
        let mut default_value = None;
        let is_variadic = false;

        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" | "field_identifier" => {
                    if name.is_none() {
                        let start = child.start_byte();
                        let end = child.end_byte();
                        name = Some(code[start..end].to_string());
                    }
                }
                "type_identifier" | "primitive_type" | "generic_type" => {
                    let start = child.start_byte();
                    let end = child.end_byte();
                    param_type = Some(code[start..end].to_string());
                }
                "default_value" => {
                    // Extract default value
                    for default_child in child.children(&mut child.walk()) {
                        let start = default_child.start_byte();
                        let end = default_child.end_byte();
                        default_value = Some(code[start..end].to_string());
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(name) = name {
            Ok(Some(Parameter {
                name,
                param_type,
                default_value,
                is_variadic,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract return type
    fn extract_return_type(node: &Node, code: &str) -> Result<Option<String>> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "return_type" || child.kind() == "type" {
                for type_child in child.children(&mut child.walk()) {
                    if type_child.kind() != "->" && type_child.kind() != ":" {
                        let start = type_child.start_byte();
                        let end = type_child.end_byte();
                        return Ok(Some(code[start..end].to_string()));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Extract visibility modifier
    fn extract_visibility(node: &Node, code: &str) -> Result<Visibility> {
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "public" => return Ok(Visibility::Public),
                "private" => return Ok(Visibility::Private),
                "protected" => return Ok(Visibility::Protected),
                "internal" => return Ok(Visibility::Internal),
                _ => {}
            }
        }
        Ok(Visibility::Private) // Default
    }

    /// Check if function is async
    fn is_async_function(node: &Node, _code: &str) -> Result<bool> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "async" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if function has generic parameters
    fn has_generic_parameters(node: &Node, _code: &str) -> Result<bool> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "type_parameters" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Analyze method signature (includes class context)
    pub fn analyze_method_signature(
        node: &Node,
        code: &str,
        class_name: Option<&str>,
    ) -> Result<MethodSignature> {
        let base_signature = Self::analyze_function_signature(node, code)?;
        let is_static = Self::is_static_method(node, code)?;
        let is_override = Self::is_override_method(node, code)?;
        let is_virtual = Self::is_virtual_method(node, code)?;
        let is_abstract = Self::is_abstract_method(node, code)?;

        Ok(MethodSignature {
            base: base_signature,
            class_name: class_name.map(|s| s.to_string()),
            is_static,
            is_override,
            is_virtual,
            is_abstract,
        })
    }

    /// Check if method is static
    fn is_static_method(node: &Node, _code: &str) -> Result<bool> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "static" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if method is override
    fn is_override_method(node: &Node, _code: &str) -> Result<bool> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "override" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if method is virtual
    fn is_virtual_method(node: &Node, _code: &str) -> Result<bool> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "virtual" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if method is abstract
    fn is_abstract_method(node: &Node, _code: &str) -> Result<bool> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "abstract" {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Generate signature string
    pub fn generate_signature_string(signature: &FunctionSignature) -> String {
        let mut result = String::new();

        // Add visibility
        match signature.visibility {
            Visibility::Public => result.push_str("pub "),
            Visibility::Private => result.push_str("priv "),
            Visibility::Protected => result.push_str("prot "),
            Visibility::Internal => result.push_str("int "),
            Visibility::Package => result.push_str("pkg "),
        }

        // Add async
        if signature.is_async {
            result.push_str("async ");
        }

        // Add function name
        result.push_str(&signature.name);

        // Add parameters
        result.push('(');
        for (i, param) in signature.parameters.iter().enumerate() {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(&param.name);
            if let Some(ref param_type) = param.param_type {
                result.push_str(": ");
                result.push_str(param_type);
            }
            if let Some(ref default) = param.default_value {
                result.push_str(" = ");
                result.push_str(default);
            }
        }
        result.push(')');

        // Add return type
        if let Some(ref return_type) = signature.return_type {
            result.push_str(" -> ");
            result.push_str(return_type);
        }

        result
    }
}

/// Function signature information
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub is_async: bool,
    pub is_generic: bool,
}

/// Method signature information (extends function signature)
#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub base: FunctionSignature,
    pub class_name: Option<String>,
    pub is_static: bool,
    pub is_override: bool,
    pub is_virtual: bool,
    pub is_abstract: bool,
}

/// Function parameter information
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub is_variadic: bool,
}

/// Visibility levels
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}
