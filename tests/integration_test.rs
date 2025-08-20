use prmt::{Token, execute, parse};
use std::env;

#[test]
fn test_basic_format() {
    let result = execute("{path}", true, None, false).expect("Failed to execute");
    assert!(!result.is_empty());
    // Should contain current directory name
    let current_dir = env::current_dir().unwrap();
    let dir_name = current_dir.file_name().unwrap().to_str().unwrap();
    assert!(result.contains(dir_name) || result.contains("~"));
}

#[test]
fn test_git_module() {
    let result = execute("{git}", true, None, false).expect("Failed to execute");
    // In a git repo, should show branch name (any non-empty string is valid)
    // The result might have a * suffix if there are uncommitted changes
    assert!(!result.is_empty(), "Git module should return a branch name");
}

#[test]
fn test_ok_fail_modules() {
    // Test with exit code 0 (ok) - use : to disable default styles
    let result = execute("{ok:}{fail:}", true, Some(0), false).expect("Failed to execute");
    assert_eq!(result, "❯"); // Only ok should show

    // Test with exit code 1 (fail) - use : to disable default styles
    let result = execute("{ok:}{fail:}", true, Some(1), false).expect("Failed to execute");
    assert_eq!(result, "❯"); // Only fail should show
}

#[test]
fn test_new_format_types() {
    // Test short version format
    let result = execute("{path::short}", true, None, false).expect("Failed to execute");
    let current_dir = env::current_dir().unwrap();
    let basename = current_dir.file_name().unwrap().to_str().unwrap();
    assert_eq!(result, basename);
}

#[test]
fn test_escape_sequences() {
    let result = execute("Line1\\nLine2\\tTab", true, None, false).expect("Failed to execute");
    assert_eq!(result, "Line1\nLine2\tTab");
}

#[test]
fn test_multiple_modules() {
    let result = execute("{path} {git}", true, None, false).expect("Failed to execute");
    assert!(!result.is_empty());
    // Should contain both path and git info if available
    let current_dir = env::current_dir().unwrap();
    let dir_name = current_dir.file_name().unwrap().to_str().unwrap();
    assert!(result.contains(dir_name) || result.contains("~"));
}

#[test]
fn test_styles() {
    // Test with styles - this should work without errors, but we won't check ANSI codes
    let formats = vec![
        "{path:cyan}",
        "{git:purple}",
        "{path:blue.bold}",
        "{rust:red}",
    ];

    for format in formats {
        let result = execute(format, true, None, false);
        assert!(result.is_ok(), "Failed to parse: {}", format);
    }
}

#[test]
fn test_prefix_suffix() {
    // Test prefix and suffix
    let formats = vec!["{path:::before:after}", "{git:::>>>:<<<}", "{ok:::[:]}"];

    for format in formats {
        let result = execute(format, true, None, false);
        assert!(result.is_ok(), "Failed to parse: {}", format);
    }
}

#[test]
fn test_rust_module() {
    let result = execute("{rust}", false, None, false).expect("Failed to execute");
    // If in a Rust project, should contain version number
    if !result.is_empty() {
        assert!(result.contains(".") || result == "rust"); // Either version or "rust" if no-version
    }
}

#[test]
fn test_custom_symbols() {
    // Test ok with custom symbol - use : to disable default styles
    let result = execute("{ok::✓}", true, Some(0), false).expect("Failed to execute");
    assert_eq!(result, "✓");

    // Test fail with custom symbol - use : to disable default styles
    let result = execute("{fail::✗}", true, Some(1), false).expect("Failed to execute");
    assert_eq!(result, "✗");

    // Test fail with code format
    let result = execute("{fail::code}", true, Some(42), false).expect("Failed to execute");
    assert_eq!(result, "42");
}

#[test]
fn test_path_formats() {
    // Test all path formats
    let result_relative =
        execute("{path::relative}", true, None, false).expect("Failed to execute");
    let result_relative_r = execute("{path::r}", true, None, false).expect("Failed to execute");
    let result_absolute =
        execute("{path::absolute}", true, None, false).expect("Failed to execute");
    let result_absolute_a = execute("{path::a}", true, None, false).expect("Failed to execute");
    let result_short = execute("{path::short}", true, None, false).expect("Failed to execute");
    let result_short_s = execute("{path::s}", true, None, false).expect("Failed to execute");
    let result_default = execute("{path}", true, None, false).expect("Failed to execute");

    // Short should be basename only
    let current_dir = env::current_dir().unwrap();
    let basename = current_dir.file_name().unwrap().to_str().unwrap();
    assert_eq!(result_short, basename);
    assert_eq!(result_short_s, basename);
    assert_eq!(result_short, result_short_s); // Short and alias should match

    // Relative formats should contain ~ if in home directory, or the basename
    assert!(result_relative.contains(basename) || result_relative.contains("~"));
    assert_eq!(result_relative, result_relative_r); // Short and long forms should match
    assert_eq!(result_relative, result_default); // Default should be relative

    // Absolute formats should never contain ~ and should always contain the basename
    assert!(!result_absolute.contains("~"));
    assert!(result_absolute.contains(basename));
    assert_eq!(result_absolute, result_absolute_a); // Short and long forms should match

    // Absolute should be different from relative if in home directory
    if result_relative.contains("~") {
        assert_ne!(result_absolute, result_relative);
    }
}

#[test]
fn test_parser_tokens() {
    // Test that parser produces correct tokens
    let tokens = parse("{path:cyan:short:[:]}");

    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "path");
            assert_eq!(params.style, "cyan");
            assert_eq!(params.format, "short");
            assert_eq!(params.prefix, "[");
            assert_eq!(params.suffix, "]");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_parser_escapes() {
    let tokens = parse("\\{not\\:placeholder\\}");
    // The parser may produce multiple text tokens due to escape processing
    let combined: String = tokens
        .iter()
        .map(|t| match t {
            Token::Text(text) => text.clone(),
            _ => panic!("Expected only text tokens"),
        })
        .collect();
    assert_eq!(combined, "{not:placeholder}");
}

#[test]
fn test_mixed_text_placeholders() {
    let tokens = parse("text {path} more {git} end");
    assert_eq!(tokens.len(), 5);

    match &tokens[0] {
        Token::Text(text) => assert_eq!(text, "text "),
        _ => panic!("Expected text token"),
    }

    match &tokens[1] {
        Token::Placeholder(params) => assert_eq!(params.module, "path"),
        _ => panic!("Expected placeholder token"),
    }

    match &tokens[2] {
        Token::Text(text) => assert_eq!(text, " more "),
        _ => panic!("Expected text token"),
    }

    match &tokens[3] {
        Token::Placeholder(params) => assert_eq!(params.module, "git"),
        _ => panic!("Expected placeholder token"),
    }

    match &tokens[4] {
        Token::Text(text) => assert_eq!(text, " end"),
        _ => panic!("Expected text token"),
    }
}
