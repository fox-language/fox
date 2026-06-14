fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

#[no_mangle]
pub extern "C" fn bench_fib() -> i32 {
    fib(30)
}
