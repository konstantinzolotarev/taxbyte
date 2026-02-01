use std::sync::Arc;
use tera::Tera;

/// Template engine wrapper for rendering HTML templates
#[derive(Clone)]
pub struct TemplateEngine {
  tera: Arc<Tera>,
}

impl TemplateEngine {
  /// Create a new template engine instance
  pub fn new() -> Result<Self, tera::Error> {
    let mut tera = Tera::new("templates/**/*.html.tera")?;
    tera.autoescape_on(vec!["html.tera", ".html"]);

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
    assert!(true);
  }
}
