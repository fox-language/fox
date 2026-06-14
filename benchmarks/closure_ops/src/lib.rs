#[no_mangle]
pub extern "C" fn bench_closure_ops() -> i32 {
    let mut sum: i32 = 0;
    let mut i: i32 = 0;
    while i < 10000 {
        let a: i32 = i;
        let b: i32 = i * 2;
        let c: i32 = i * 3;
        let compute = |x: i32| -> i32 {
            (x * a) - b + c
        };
        sum = (sum + compute(i)) % 99991;
        i = i + 1;
    }
    sum
}
