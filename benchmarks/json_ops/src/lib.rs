use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: i32,
    is_student: bool,
    height: f64,
}

#[no_mangle]
pub extern "C" fn bench_json_serialize() -> i32 {
    let p = Person {
        name: "Alice".into(),
        age: 30,
        is_student: false,
        height: 5.8,
    };
    let mut sum: i32 = 0;
    let mut i = 0;
    while i < 500 {
        let json = serde_json::to_string(&p).unwrap();
        sum += json.len() as i32;
        i += 1;
    }
    sum
}

#[no_mangle]
pub extern "C" fn bench_json_deserialize() -> i32 {
    let raw = r#"{"name":"Bob","age":25,"is_student":true,"height":6.0}"#;
    let mut sum: i32 = 0;
    let mut i = 0;
    while i < 500 {
        let person: Person = serde_json::from_str(raw).unwrap();
        sum += person.age;
        i += 1;
    }
    sum
}
