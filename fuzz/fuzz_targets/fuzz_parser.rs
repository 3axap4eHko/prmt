#![no_main]
use libfuzzer_sys::fuzz_target;
use prmt::parse;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz the parser with arbitrary UTF-8 input
        let _ = parse(s);
    }
});