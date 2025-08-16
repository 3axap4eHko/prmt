use prmt::parser::Parser;

fn analyze_template(template: &str) {
    let parser = Parser::new(template);
    let tokens = parser.parse();
    
    let open_count = template.matches('{').count();
    let starts_with_open = template.starts_with('{');
    
    let estimated = if open_count == 0 {
        1
    } else if !starts_with_open {
        1 + (open_count * 2)
    } else {
        open_count * 2
    };
    
    let accuracy = if tokens.len() <= estimated {
        format!("✓ {}%", (tokens.len() as f64 / estimated as f64 * 100.0) as i32)
    } else {
        format!("✗ Under by {}", tokens.len() - estimated)
    };
    
    println!("{:40} | Tokens: {:2} | Est: {:2} | {}", 
             if template.len() > 40 { format!("{}...", &template[..37]) } else { template.to_string() },
             tokens.len(),
             estimated,
             accuracy);
}

fn main() {
    println!("Template Capacity Analysis:");
    println!("{:40} | {:10} | {:6} | Accuracy", "Template", "Actual", "Est");
    println!("{:-<70}", "");
    
    let templates = vec![
        // Pure text
        "Plain text without placeholders",
        "",
        
        // Starting with placeholder
        "{path}",
        "{path} {git} {rust}",
        "{a}{b}{c}{d}{e}",
        
        // Starting with text
        "Start {path} end",
        "Start {path} middle {git} end",
        "Text {one}",
        "Long prefix text {path:cyan} {git}",
        
        // Complex real-world
        "{path:cyan:short:[:]} on {git:yellow::🌿 :}",
        "$ {user}@{host}:{path:short} {git} > ",
        
        // Edge cases
        "{",
        "}",
        "\\{escaped\\} {real}",
        "{unclosed",
    ];
    
    for template in templates {
        analyze_template(template);
    }
    
    println!("\n✓ = Exact or slight over-allocation (good)");
    println!("✗ = Under-allocation (requires reallocation)");
}