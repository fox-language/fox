use std::collections::HashMap;

#[no_mangle]
pub extern "C" fn bench_map_ops() -> i32 {
    let mut m: HashMap<&str, i32> = HashMap::with_capacity(64);
    m.insert("alpha", 1);
    m.insert("beta", 2);
    m.insert("gamma", 3);
    m.insert("delta", 4);
    m.insert("epsilon", 5);
    m.insert("zeta", 6);
    m.insert("eta", 7);
    m.insert("theta", 8);
    m.insert("iota", 9);
    m.insert("kappa", 10);

    let mut sum: i32 = 0;
    let mut i: i32 = 0;
    while i < 200 {
        if let Some(&v) = m.get("alpha") {
            sum += v;
        }
        if let Some(&v) = m.get("gamma") {
            sum += v * 2;
        }
        if let Some(&v) = m.get("epsilon") {
            sum += v * 3;
        }
        if m.contains_key("zeta") {
            sum += 6;
        }
        if m.contains_key("missing") {
            sum += 1000;
        }

        m.insert("alpha", sum % 256);
        m.insert("kappa", i);

        i += 1;
    }
    sum + m.len() as i32
}
