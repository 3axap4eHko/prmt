use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use std::env;

pub struct EnvModule;

impl Default for EnvModule {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for EnvModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        if format.is_empty() {
            return Err(PromptError::InvalidFormat {
                module: "env".to_string(),
                format: format.to_string(),
                valid_formats: "Provide an environment variable name, e.g., {env:blue:USER}"
                    .to_string(),
            });
        }

        match env::var_os(format) {
            None => Ok(None),
            Some(value) if value.is_empty() => Ok(None),
            Some(value) => Ok(Some(value.to_string_lossy().into_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::ffi::OsString;

    struct EnvVarGuard {
        key: String,
        original: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &str, value: &str) -> Self {
            let original = env::var_os(key);
            unsafe {
                env::set_var(key, value);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }

        fn unset(key: &str) -> Self {
            let original = env::var_os(key);
            unsafe {
                env::remove_var(key);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                unsafe {
                    env::set_var(&self.key, value);
                }
            } else {
                unsafe {
                    env::remove_var(&self.key);
                }
            }
        }
    }

    #[test]
    #[serial]
    fn renders_value_when_variable_is_present() {
        let module = EnvModule::new();
        let _guard = EnvVarGuard::set("PRMT_TEST_ENV_PRESENT", "zenpie");

        let value = module
            .render("PRMT_TEST_ENV_PRESENT", &ModuleContext::default())
            .unwrap();

        assert_eq!(value, Some("zenpie".to_string()));
    }

    #[test]
    #[serial]
    fn returns_none_when_variable_missing() {
        let module = EnvModule::new();
        let _guard = EnvVarGuard::unset("PRMT_TEST_ENV_MISSING");

        let value = module
            .render("PRMT_TEST_ENV_MISSING", &ModuleContext::default())
            .unwrap();

        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn returns_none_when_variable_empty() {
        let module = EnvModule::new();
        let _guard = EnvVarGuard::set("PRMT_TEST_ENV_EMPTY", "");

        let value = module
            .render("PRMT_TEST_ENV_EMPTY", &ModuleContext::default())
            .unwrap();

        assert_eq!(value, None);
    }

    #[test]
    fn errors_when_format_missing() {
        let module = EnvModule::new();
        let err = module.render("", &ModuleContext::default()).unwrap_err();

        match err {
            PromptError::InvalidFormat { module, .. } => assert_eq!(module, "env"),
            other => panic!("expected invalid format error, got {other:?}"),
        }
    }
}
