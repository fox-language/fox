export function bench_map_ops() {
    const m = new Map();
    m.set("alpha", 1);
    m.set("beta", 2);
    m.set("gamma", 3);
    m.set("delta", 4);
    m.set("epsilon", 5);
    m.set("zeta", 6);
    m.set("eta", 7);
    m.set("theta", 8);
    m.set("iota", 9);
    m.set("kappa", 10);

    let sum = 0;
    let i = 0;
    while (i < 200) {
        const a = m.get("alpha");
        if (a !== undefined) sum += a;
        const c = m.get("gamma");
        if (c !== undefined) sum += c * 2;
        const e = m.get("epsilon");
        if (e !== undefined) sum += e * 3;
        if (m.has("zeta")) sum += 6;
        if (m.has("missing")) sum += 1000;
        m.set("alpha", sum % 256);
        m.set("kappa", i);
        i++;
    }
    return sum + m.size;
}
