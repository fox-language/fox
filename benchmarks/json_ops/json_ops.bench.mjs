export function bench_json_serialize() {
    const p = { name: "Alice", age: 30, is_student: false, height: 5.8 };
    let sum = 0;
    let i = 0;
    while (i < 500) {
        const json = JSON.stringify(p);
        sum += json.length;
        i++;
    }
    return sum;
}

export function bench_json_deserialize() {
    const raw = '{"name":"Bob","age":25,"is_student":true,"height":6.0}';
    let sum = 0;
    let i = 0;
    while (i < 500) {
        const person = JSON.parse(raw);
        sum += person.age;
        i++;
    }
    return sum;
}
