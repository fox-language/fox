#[no_mangle]
pub extern "C" fn bench_math_complex() -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: f64 = 1.0;
    while i <= 1000.0 {
        sum += i.sin() * i.cos() + i.sqrt() + i.ln();
        i += 1.0;
    }
    sum
}
