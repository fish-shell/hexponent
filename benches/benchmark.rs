use criterion::{black_box, criterion_group, criterion_main, Criterion};

use hexponent::FloatLiteral;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("parsing", |b| {
        b.iter(|| FloatLiteral::from_bytes(black_box(b"0xabc.defp123")));
    });
    c.bench_function("convert f64", |b| {
        let literal = FloatLiteral::from_bytes(black_box(b"0xabc.defp123")).unwrap();
        b.iter(move || literal.clone().convert::<f64>())
    });
    c.bench_function("convert f32", |b| {
        let literal = FloatLiteral::from_bytes(black_box(b"0xabc.defp123")).unwrap();
        b.iter(move || literal.clone().convert::<f32>())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
