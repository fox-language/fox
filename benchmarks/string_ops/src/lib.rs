#[no_mangle]
pub extern "C" fn bench_string_ops() -> i32 {
    let s = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.";
    let mut sum: i32 = 0;
    let mut i: i32 = 0;
    while i < 100 {
        if s.starts_with("Lorem") {
            sum += 1;
        }
        if s.ends_with("consequat.") {
            sum += 2;
        }
        if s.contains("tempor") {
            sum += 3;
        }
        if let Some(idx) = s.find("adipiscing") {
            sum += idx as i32;
        } else {
            sum -= 1;
        }
        if let Some(last_idx) = s.rfind("ut") {
            sum += last_idx as i32;
        } else {
            sum -= 1;
        }
        i += 1;
    }
    sum
}
