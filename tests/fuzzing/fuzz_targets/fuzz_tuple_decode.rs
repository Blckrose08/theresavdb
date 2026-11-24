#![no_main]
use libfuzzer_sys::fuzz_target;
use theresav_page::Tuple;

fuzz_target!(|data: &[u8]| {
    // Tuple::deserialize must gracefully return Ok or Err, NEVER panic
    let _ = Tuple::deserialize(data);
});
