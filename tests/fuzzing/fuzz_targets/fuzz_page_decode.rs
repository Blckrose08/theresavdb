#![no_main]
use libfuzzer_sys::fuzz_target;
use theresav_page::Page;

fuzz_target!(|data: &[u8]| {
    // Page::from_bytes must gracefully return Ok or Err, NEVER panic
    let _ = Page::from_bytes(data);
});
