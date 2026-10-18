#![no_main]

use libfuzzer_sys::fuzz_target;
use mm_protocol::parse_message;

fuzz_target!(|bytes: &[u8]| {
    let _ = parse_message(bytes);
});
