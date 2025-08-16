use prmt::{parse, Token};

#[test]
fn test_empty_string() {
    let tokens = parse("");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_plain_text() {
    let tokens = parse("plain text without placeholders");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::Text(text) => assert_eq!(text, "plain text without placeholders"),
        _ => panic!("Expected text token"),
    }
}

#[test]
fn test_single_placeholder_minimal() {
    let tokens = parse("{module}");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "");
            assert_eq!(params.format, "");
            assert_eq!(params.prefix, "");
            assert_eq!(params.suffix, "");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_single_placeholder_with_style() {
    let tokens = parse("{module:cyan}");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "cyan");
            assert_eq!(params.format, "");
            assert_eq!(params.prefix, "");
            assert_eq!(params.suffix, "");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_single_placeholder_with_format() {
    let tokens = parse("{module::short}");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "");
            assert_eq!(params.format, "short");
            assert_eq!(params.prefix, "");
            assert_eq!(params.suffix, "");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_single_placeholder_with_prefix_suffix() {
    let tokens = parse("{module:::before:after}");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "");
            assert_eq!(params.format, "");
            assert_eq!(params.prefix, "before");
            assert_eq!(params.suffix, "after");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_placeholder_all_fields() {
    let tokens = parse("{module:red.bold:short:[:]}");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "red.bold");
            assert_eq!(params.format, "short");
            assert_eq!(params.prefix, "[");
            assert_eq!(params.suffix, "]");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_multiple_placeholders() {
    let tokens = parse("{path}{git}{rust}");
    assert_eq!(tokens.len(), 3);
    
    match &tokens[0] {
        Token::Placeholder(params) => assert_eq!(params.module, "path"),
        _ => panic!("Expected placeholder token"),
    }
    
    match &tokens[1] {
        Token::Placeholder(params) => assert_eq!(params.module, "git"),
        _ => panic!("Expected placeholder token"),
    }
    
    match &tokens[2] {
        Token::Placeholder(params) => assert_eq!(params.module, "rust"),
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_mixed_text_and_placeholders() {
    let tokens = parse("start {path} middle {git} end");
    assert_eq!(tokens.len(), 5);
    
    match &tokens[0] {
        Token::Text(text) => assert_eq!(text, "start "),
        _ => panic!("Expected text token"),
    }
    
    match &tokens[1] {
        Token::Placeholder(params) => assert_eq!(params.module, "path"),
        _ => panic!("Expected placeholder token"),
    }
    
    match &tokens[2] {
        Token::Text(text) => assert_eq!(text, " middle "),
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

#[test]
fn test_escape_sequences() {
    // Test escaped braces
    let tokens = parse("\\{not a placeholder\\}");
    let text = tokens.iter().map(|t| match t {
        Token::Text(s) => s.as_str(),
        _ => panic!("Expected text token"),
    }).collect::<String>();
    assert_eq!(text, "{not a placeholder}");
    
    // Test escaped colon
    let tokens = parse("time\\: 12\\:30");
    let text = tokens.iter().map(|t| match t {
        Token::Text(s) => s.as_str(),
        _ => panic!("Expected text token"),
    }).collect::<String>();
    assert_eq!(text, "time: 12:30");
    
    // Test escaped backslash
    let tokens = parse("path\\\\to\\\\file");
    let text = tokens.iter().map(|t| match t {
        Token::Text(s) => s.as_str(),
        _ => panic!("Expected text token"),
    }).collect::<String>();
    assert_eq!(text, "path\\to\\file");
    
    // Test newline and tab
    let tokens = parse("line1\\nline2\\ttab");
    let text = tokens.iter().map(|t| match t {
        Token::Text(s) => s.as_str(),
        _ => panic!("Expected text token"),
    }).collect::<String>();
    assert_eq!(text, "line1\nline2\ttab");
}


#[test]
fn test_special_characters_in_fields() {
    // Test special chars in prefix/suffix
    let tokens = parse("{module:::>>>:<<<}");
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.prefix, ">>>");
            assert_eq!(params.suffix, "<<<");
        }
        _ => panic!("Expected placeholder token"),
    }
    
    // Test spaces in prefix/suffix
    let tokens = parse("{module::: on : }");
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.prefix, " on ");
            assert_eq!(params.suffix, " ");
        }
        _ => panic!("Expected placeholder token"),
    }
    
    // Test symbols as format (for ok/fail modules)
    let tokens = parse("{ok::✓}");
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "ok");
            assert_eq!(params.format, "✓");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_complex_styles() {
    let tokens = parse("{module:cyan.bold.italic}");
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.style, "cyan.bold.italic");
        }
        _ => panic!("Expected placeholder token"),
    }
    
    let tokens = parse("{module:#ff0000.bold}");
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.style, "#ff0000.bold");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_empty_fields() {
    // All fields empty except module
    let tokens = parse("{module::::}");
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "");
            assert_eq!(params.format, "");
            assert_eq!(params.prefix, "");
            assert_eq!(params.suffix, "");
        }
        _ => panic!("Expected placeholder token"),
    }
    
    // Style and format empty, but prefix/suffix present
    let tokens = parse("{module:::A:B}");
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "");
            assert_eq!(params.format, "");
            assert_eq!(params.prefix, "A");
            assert_eq!(params.suffix, "B");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_nested_braces_not_allowed() {
    // Parser should handle nested braces as text
    let tokens = parse("{module:{nested}}");
    // This should parse as placeholder with module "module" and style "{nested"
    match &tokens[0] {
        Token::Placeholder(params) => {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "{nested");
        }
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_unclosed_placeholder() {
    // Unclosed placeholder should be treated as text
    let tokens = parse("{unclosed");
    let text = tokens.iter().map(|t| match t {
        Token::Text(s) => s.as_str(),
        _ => panic!("Expected text token"),
    }).collect::<String>();
    assert_eq!(text, "{unclosed");
}

#[test]
fn test_consecutive_placeholders() {
    let tokens = parse("{a}{b}{c}");
    assert_eq!(tokens.len(), 3);
    
    match &tokens[0] {
        Token::Placeholder(params) => assert_eq!(params.module, "a"),
        _ => panic!("Expected placeholder token"),
    }
    
    match &tokens[1] {
        Token::Placeholder(params) => assert_eq!(params.module, "b"),
        _ => panic!("Expected placeholder token"),
    }
    
    match &tokens[2] {
        Token::Placeholder(params) => assert_eq!(params.module, "c"),
        _ => panic!("Expected placeholder token"),
    }
}

#[test]
fn test_real_world_formats() {
    // Common prompt format
    let tokens = parse("{path:cyan} {git:purple} {rust:red}");
    assert_eq!(tokens.len(), 5);
    
    // With prefix/suffix
    let tokens = parse("{ok:green:✓:[:]} {fail:red:✗:[:]}");
    assert_eq!(tokens.len(), 3);
    
    // Complex format
    let tokens = parse("{path:blue.bold:tilde} on {git:yellow::🌿 :} {rust:::v:}");
    assert!(tokens.len() > 0);
}

#[test]
fn test_whitespace_preservation() {
    let tokens = parse("  spaces  {module}  more spaces  ");
    
    match &tokens[0] {
        Token::Text(text) => assert_eq!(text, "  spaces  "),
        _ => panic!("Expected text token"),
    }
    
    match &tokens[2] {
        Token::Text(text) => assert_eq!(text, "  more spaces  "),
        _ => panic!("Expected text token"),
    }
}

#[test]
fn test_colon_in_text() {
    // Colons outside placeholders should be preserved
    let tokens = parse("time: {module} at: location");
    
    match &tokens[0] {
        Token::Text(text) => assert_eq!(text, "time: "),
        _ => panic!("Expected text token"),
    }
    
    match &tokens[2] {
        Token::Text(text) => assert_eq!(text, " at: location"),
        _ => panic!("Expected text token"),
    }
}

#[test]
fn test_escape_in_placeholder_fields() {
    // Escapes should work within placeholder fields
    let tokens = parse("{module:::a\\:b:c\\:d}");
    match &tokens[0] {
        Token::Placeholder(params) => {
            // The parser should handle escapes in prefix/suffix
            assert_eq!(params.prefix, "a:b");
            assert_eq!(params.suffix, "c:d");
        }
        _ => panic!("Expected placeholder token"),
    }
}