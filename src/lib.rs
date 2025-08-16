pub mod module_trait;
pub mod style;
pub mod registry;
pub mod parser;
pub mod executor;
pub mod modules;

// Re-export main types and functions
pub use module_trait::{Module, ModuleContext};
pub use style::{ModuleStyle, AnsiStyle};
pub use registry::ModuleRegistry;
pub use parser::{parse, Token, Params};
pub use executor::{execute, render_template};