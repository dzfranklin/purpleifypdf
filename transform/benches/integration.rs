// use criterion::{criterion_group, criterion_main, Criterion};
// use std::{fs, time::Duration};

// use purpleifypdf::{transform, PageRange, Quality};

// fn bench(c: &mut Criterion) {
//     let in_blob = fs::read("test_assets/multipage_test.pdf").unwrap();
//     let pages = PageRange {
//         starting_index: 0,
//         count: 1,
//     };
//     let quality = Quality::Normal;

//     c.bench_function("multipage", |b| {
//         b.iter(|| transform(in_blob.clone(), Some(pages), quality, None).unwrap().finish())
//     });
// }

// criterion_group!{
//     name = benches;
//     config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(180));
//     targets = bench
// }
// criterion_main!(benches);

fn main() {}