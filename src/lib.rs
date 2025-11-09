pub mod detector;
pub mod error;
pub mod executor;
pub mod memo;
pub mod module_trait;
pub mod modules;
pub mod parser;
pub mod registry;
pub mod style;
pub mod template;

// Re-export main types and functions
pub use error::{PromptError, Result};
pub use executor::{execute, render_template};
pub use module_trait::{Module, ModuleContext};
pub use parser::{Params, Token, parse};
pub use registry::ModuleRegistry;
pub use style::{AnsiStyle, ModuleStyle};
pub use template::Template;
