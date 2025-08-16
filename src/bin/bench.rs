use std::time::Instant;
use prmt::execute;

fn main() {
    let templates = vec![
        ("simple", "{path}"),
        ("complex", "{path:cyan:short:[:]} on {git:yellow::🌿 :} {rust::full}"),
        ("real_world", "{path:cyan} {rust:red} {git:purple} {ok:green:✓} {fail:red:✗}"),
    ];
    
    println!("Parser Performance (with fast memchr-based implementation):\n");
    
    for (name, template) in templates {
        // Warm up
        for _ in 0..100 {
            let _ = execute(template, true, None);
        }
        
        // Measure
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = execute(template, true, None);
        }
        let time = start.elapsed();
        
        println!("{:12} {:?} ({:.2} µs/iter)", 
            format!("{}:", name),
            time, 
            time.as_nanos() as f64 / 10000.0 / 1000.0
        );
    }
}