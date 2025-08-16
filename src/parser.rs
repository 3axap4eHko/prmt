use std::borrow::Cow;
use memchr;

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub module: String,
    pub style: String,
    pub format: String,
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Text(String),
    Placeholder(Params),
}

pub struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
        }
    }
    
    fn skip_to(&mut self, pos: usize) {
        self.pos = pos.min(self.bytes.len());
    }
    
    fn current_slice(&self, start: usize) -> &'a str {
        unsafe {
            std::str::from_utf8_unchecked(&self.bytes[start..self.pos])
        }
    }
    
    fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }
    
    pub fn parse(mut self) -> Vec<Token> {
        // Pre-allocate capacity based on open brace count
        let open_count = memchr::memchr_iter(b'{', self.bytes).count();
        
        let capacity = if open_count == 0 {
            1  // Pure text, single token
        } else if self.bytes.get(0) != Some(&b'{') {
            1 + (open_count * 2)  // Has leading text
        } else {
            open_count * 2  // Starts with placeholder
        };
        
        let mut tokens = Vec::with_capacity(capacity);
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }
    
    fn next_token(&mut self) -> Option<Token> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        
        let start = self.pos;
        
        if let Some(offset) = memchr::memchr3(b'{', b'\\', b'}', self.remaining()) {
            let abs_pos = self.pos + offset;
            match self.bytes[abs_pos] {
                b'\\' => {
                    if abs_pos + 1 < self.bytes.len() {
                        match self.bytes[abs_pos + 1] {
                            b'{' | b'}' | b'\\' | b'n' | b't' | b':' => {
                                if abs_pos > start {
                                    self.skip_to(abs_pos);
                                    return Some(Token::Text(self.current_slice(start).to_string()));
                                }
                                
                                let escaped = match self.bytes[abs_pos + 1] {
                                    b'n' => "\n",
                                    b't' => "\t",
                                    b'\\' => "\\",
                                    b'{' => "{",
                                    b'}' => "}",
                                    b':' => ":",
                                    _ => unreachable!(),
                                };
                                self.skip_to(abs_pos + 2);
                                return Some(Token::Text(escaped.to_string()));
                            }
                            _ => {
                                self.skip_to(abs_pos + 2);
                                return self.next_token();
                            }
                        }
                    } else {
                        self.skip_to(self.bytes.len());
                        if start < self.bytes.len() {
                            return Some(Token::Text(self.current_slice(start).to_string()));
                        }
                        return None;
                    }
                }
                b'{' => {
                    if abs_pos > start {
                        self.skip_to(abs_pos);
                        return Some(Token::Text(self.current_slice(start).to_string()));
                    }
                    
                    if let Some(end_offset) = memchr::memchr(b'}', &self.bytes[abs_pos + 1..]) {
                        let end_pos = abs_pos + 1 + end_offset;
                        let content = &self.bytes[abs_pos + 1..end_pos];
                        
                        if let Some(params) = parse_placeholder(unsafe { std::str::from_utf8_unchecked(content) }) {
                            self.skip_to(end_pos + 1);
                            return Some(Token::Placeholder(params));
                        }
                    }
                    
                    self.skip_to(abs_pos + 1);
                    return Some(Token::Text("{".to_string()));
                }
                b'}' => {
                    if abs_pos > start {
                        self.skip_to(abs_pos);
                        return Some(Token::Text(self.current_slice(start).to_string()));
                    }
                    self.skip_to(abs_pos + 1);
                    return Some(Token::Text("}".to_string()));
                }
                _ => unreachable!(),
            }
        } else {
            self.skip_to(self.bytes.len());
            if start < self.bytes.len() {
                return Some(Token::Text(self.current_slice(start).to_string()));
            }
        }
        
        None
    }
}

fn parse_placeholder(content: &str) -> Option<Params> {
    let fields = split_fields(content);
    
    if fields[0].is_empty() {
        return None;
    }
    
    Some(Params {
        module: unescape_if_needed(&fields[0]).into_owned(),
        style: unescape_if_needed(&fields[1]).into_owned(),
        format: unescape_if_needed(&fields[2]).into_owned(),
        prefix: unescape_if_needed(&fields[3]).into_owned(),
        suffix: unescape_if_needed(&fields[4]).into_owned(),
    })
}

fn split_fields(s: &str) -> [&str; 5] {
    let mut fields = [""; 5];
    let mut field_idx = 0;
    let mut start = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    
    while i < bytes.len() && field_idx < 4 {
        if bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i] == b':' {
            fields[field_idx] = unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) };
            field_idx += 1;
            start = i + 1;
            i += 1;
        } else {
            i += 1;
        }
    }
    
    fields[field_idx] = unsafe { std::str::from_utf8_unchecked(&bytes[start..]) };
    fields
}

fn unescape_if_needed(s: &str) -> Cow<'_, str> {
    if !s.contains('\\') {
        return Cow::Borrowed(s);
    }
    
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    ':' => result.push(':'),
                    '{' => result.push('{'),
                    '}' => result.push('}'),
                    _ => {
                        result.push('\\');
                        result.push(next);
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(ch);
        }
    }
    
    Cow::Owned(result)
}

pub fn parse(template: &str) -> Vec<Token> {
    Parser::new(template).parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_text() {
        let tokens = parse("Hello, World!");
        assert_eq!(tokens, vec![Token::Text("Hello, World!".to_string())]);
    }
    
    #[test]
    fn test_simple_placeholder() {
        let tokens = parse("{path}");
        assert_eq!(tokens.len(), 1);
        if let Token::Placeholder(params) = &tokens[0] {
            assert_eq!(params.module, "path");
            assert_eq!(params.style, "");
            assert_eq!(params.format, "");
            assert_eq!(params.prefix, "");
            assert_eq!(params.suffix, "");
        } else {
            panic!("Expected placeholder");
        }
    }
    
    #[test]
    fn test_placeholder_with_all_fields() {
        let tokens = parse("{path:cyan:short:[:]}");
        assert_eq!(tokens.len(), 1);
        if let Token::Placeholder(params) = &tokens[0] {
            assert_eq!(params.module, "path");
            assert_eq!(params.style, "cyan");
            assert_eq!(params.format, "short");
            assert_eq!(params.prefix, "[");
            assert_eq!(params.suffix, "]");
        } else {
            panic!("Expected placeholder");
        }
    }
    
    #[test]
    fn test_escaped_colon_in_field() {
        let tokens = parse("{module:style:format:pre\\:fix:suffix}");
        if let Token::Placeholder(params) = &tokens[0] {
            assert_eq!(params.prefix, "pre:fix");
        } else {
            panic!("Expected placeholder");
        }
    }
    
    #[test]
    fn test_escaped_braces_in_text() {
        let tokens = parse("\\{not a placeholder\\}");
        assert_eq!(tokens, vec![
            Token::Text("{".to_string()),
            Token::Text("not a placeholder".to_string()),
            Token::Text("}".to_string()),
        ]);
    }
    
    #[test]
    fn test_escape_sequences() {
        let tokens = parse("Line1\\nLine2\\tTabbed");
        assert_eq!(tokens, vec![
            Token::Text("Line1".to_string()),
            Token::Text("\n".to_string()),
            Token::Text("Line2".to_string()),
            Token::Text("\t".to_string()),
            Token::Text("Tabbed".to_string()),
        ]);
    }
    
    #[test]
    fn test_unclosed_placeholder() {
        let tokens = parse("{unclosed");
        // The parser should treat unclosed placeholders as text
        let combined: String = tokens.iter().map(|t| match t {
            Token::Text(s) => s.as_str(),
            _ => panic!("Expected text token"),
        }).collect();
        assert_eq!(combined, "{unclosed");
    }
    
    #[test]
    fn test_empty_fields() {
        let tokens = parse("{module::::}");
        if let Token::Placeholder(params) = &tokens[0] {
            assert_eq!(params.module, "module");
            assert_eq!(params.style, "");
            assert_eq!(params.format, "");
            assert_eq!(params.prefix, "");
            assert_eq!(params.suffix, "");
        } else {
            panic!("Expected placeholder");
        }
    }
    
    #[test]
    fn test_mixed_content() {
        let tokens = parse("Hello {user:yellow}, welcome to {path:cyan:short}!");
        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[0], Token::Text(_)));
        assert!(matches!(tokens[1], Token::Placeholder(_)));
        assert!(matches!(tokens[2], Token::Text(_)));
        assert!(matches!(tokens[3], Token::Placeholder(_)));
        assert!(matches!(tokens[4], Token::Text(_)));
    }
}
