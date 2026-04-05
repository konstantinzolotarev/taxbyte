use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Tera, Value, to_value, try_get_value};

/// Template engine wrapper for rendering HTML templates
#[derive(Clone)]
pub struct TemplateEngine {
  tera: Arc<Tera>,
}

/// Custom Tera filter to zero-pad a number to 2 digits (e.g. 3 → "03")
fn zero_pad(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
  let num = try_get_value!("zero_pad", "value", u64, value);
  Ok(to_value(format!("{:02}", num))?)
}

/// Custom Tera filter to get absolute value of a decimal string
fn abs_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
  let decimal_str = try_get_value!("abs", "value", String, value);
  let decimal: Decimal = decimal_str
    .parse()
    .map_err(|e| tera::Error::msg(format!("Invalid decimal value: {}", e)))?;
  Ok(to_value(decimal.abs().to_string())?)
}

/// Custom Tera filter to format Decimal values as money
/// Rounds to 2 decimal places, removes .00 if zero cents
fn format_money(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
  let decimal_str = try_get_value!("format_money", "value", String, value);

  // Parse the string as Decimal
  let decimal: Decimal = decimal_str
    .parse()
    .map_err(|e| tera::Error::msg(format!("Invalid decimal value: {}", e)))?;

  // Round to 2 decimal places
  let rounded = decimal.round_dp(2);

  // Convert to string and check if it ends with .00
  let formatted = format!("{:.2}", rounded);
  let result = if formatted.ends_with(".00") {
    // Remove .00 for whole numbers
    formatted.trim_end_matches(".00").to_string()
  } else {
    formatted
  };

  Ok(to_value(result)?)
}

impl TemplateEngine {
  /// Create a new template engine instance
  pub fn new() -> Result<Self, tera::Error> {
    let mut tera = Tera::new("templates/**/*.html.tera")?;
    tera.autoescape_on(vec!["html.tera", ".html"]);

    // Register custom filters
    tera.register_filter("format_money", format_money);
    tera.register_filter("zero_pad", zero_pad);
    tera.register_filter("abs", abs_filter);

    Ok(Self {
      tera: Arc::new(tera),
    })
  }

  /// Render a template with the given context
  pub fn render(&self, template: &str, context: &tera::Context) -> Result<String, tera::Error> {
    self.tera.render(template, context)
  }

  /// Render a template with no context
  pub fn render_simple(&self, template: &str) -> Result<String, tera::Error> {
    self.tera.render(template, &tera::Context::new())
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn test_template_engine_creation() {
    // In test environment, templates might not exist
    // This test just ensures the structure compiles
  }
}
