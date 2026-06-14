#[no_mangle]
pub extern "C" fn bench_vec_ops() -> i32 {
    let mut v: Vec<i32> = Vec::new();
    let mut i: i32 = 0;
    while i < 10000 {
        v.push(i);
        i += 1;
    }
    let mut sum: i32 = 0;
    i = 0;
    while i < 10000 {
        if let Some(&x) = v.get(i as usize) {
            v[i as usize] = x * 2;
            sum += x;
        }
        i += 1;
    }
    i = 0;
    while i < 5000 {
        if let Some(x) = v.pop() {
            sum -= x;
        }
        i += 1;
    }
    sum + v.len() as i32
}
