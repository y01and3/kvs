use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use kvs::{KvStore, KvsEngine, SledKvsEngine};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tempfile::TempDir;

fn write_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_bench");
    group.bench_function("kvs", move |b| {
        b.iter_batched(
            || {
                let kvs = KvStore::open(TempDir::new().unwrap().path()).unwrap();
                let keys = (0..100)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                let values = (0..100)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                (kvs, keys, values)
            },
            |(mut kvs, keys, values)| {
                for i in 0..100 {
                    kvs.set(keys[i].clone(), values[i].clone()).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("sled", move |b| {
        b.iter_batched(
            || {
                let path = TempDir::new().unwrap();
                let sled = SledKvsEngine::new(sled::open(&path).unwrap());
                let keys = (0..100)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                let values = (0..100)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                (sled, keys, values, path)
            },
            |(mut sled, keys, values, _path)| {
                for i in 0..100 {
                    sled.set(keys[i].clone(), values[i].clone()).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn read_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_bench");
    group.bench_function("kvs", move |b| {
        b.iter_batched(
            || {
                let path = TempDir::new().unwrap();
                let mut kvs = KvStore::open(path.path()).unwrap();
                let keys = (0..10000)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                let values = (0..10000)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                for i in 0..10000 {
                    kvs.set(keys[i].clone(), values[i].clone()).unwrap();
                }
                drop(kvs);
                (KvStore::open(path.path()).unwrap(), keys)
            },
            |(mut kvs, keys)| {
                for _ in 0..1000 {
                    kvs.get(keys[thread_rng().gen_range(0, 10000)].clone())
                        .unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("sled", move |b| {
        b.iter_batched(
            || {
                let path = TempDir::new().unwrap();
                let mut sled = SledKvsEngine::new(sled::open(&path).unwrap());
                let keys = (0..10000)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                let values = (0..10000)
                    .map(|_| thread_rng().gen_range(1, 10000))
                    .map(|i| {
                        thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(i)
                            .map(char::from)
                            .collect()
                    })
                    .collect::<Vec<String>>();
                for i in 0..10000 {
                    sled.set(keys[i].clone(), values[i].clone()).unwrap();
                }
                drop(sled);
                (SledKvsEngine::new(sled::open(&path).unwrap()), keys, path)
            },
            |(mut sled, keys, _path)| {
                for _ in 0..1000 {
                    sled.get(keys[thread_rng().gen_range(0, 10000)].clone())
                        .unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, write_benches, read_benches);
criterion_main!(benches);
