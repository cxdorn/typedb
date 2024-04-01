/*
 * Copyright (C) 2023 Vaticle
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

#![deny(unused_must_use)]

use std::{fs::File, os::raw::c_int, path::Path};

use bytes::{byte_array::ByteArray, byte_reference::ByteReference};
use criterion::{criterion_group, criterion_main, profiler::Profiler, Criterion};
use durability::wal::WAL;
use pprof::ProfilerGuard;
use primitive::prefix_range::PrefixRange;
use resource::constants::snapshot::{BUFFER_KEY_INLINE, BUFFER_VALUE_INLINE};
use storage::{
    key_value::{StorageKey, StorageKeyArray, StorageKeyReference},
    KeyspaceSet, MVCCStorage,
};
use test_utils::{create_tmp_dir, init_logging};

macro_rules! test_keyspace_set {
    {$($variant:ident => $id:literal : $name: literal),* $(,)?} => {
        #[derive(Copy, Clone)]
        enum TestKeyspaceSet { $($variant),* }
        impl KeyspaceSet for TestKeyspaceSet {
            fn iter() -> impl Iterator<Item = Self> { [$(Self::$variant),*].into_iter() }
            fn id(&self) -> u8 {
                match *self { $(Self::$variant => $id),* }
            }
            fn name(&self) -> &'static str {
                match *self { $(Self::$variant => $name),* }
            }
        }
    };
}

test_keyspace_set! {
    Keyspace => 0: "keyspace",
}
use self::TestKeyspaceSet::Keyspace;

fn random_key_24(keyspace: TestKeyspaceSet) -> StorageKeyArray<BUFFER_KEY_INLINE> {
    let mut bytes: [u8; 24] = rand::random();
    bytes[0] = 0b0;
    StorageKeyArray::from((bytes.as_slice(), keyspace))
}

fn random_key_4(keyspace: TestKeyspaceSet) -> StorageKeyArray<BUFFER_KEY_INLINE> {
    let mut bytes: [u8; 4] = rand::random();
    bytes[0] = 0b0;
    StorageKeyArray::from((bytes.as_slice(), keyspace))
}

fn populate_storage(storage: &MVCCStorage<WAL>, keyspace: TestKeyspaceSet, key_count: usize) -> usize {
    const BATCH_SIZE: usize = 1_000;
    let mut snapshot = storage.open_snapshot_write();
    for i in 0..key_count {
        if i % BATCH_SIZE == 0 {
            snapshot.commit().unwrap();
            snapshot = storage.open_snapshot_write();
        }
        snapshot.put(random_key_24(keyspace)).unwrap();
    }
    snapshot.commit().unwrap();
    println!("Keys written: {}", key_count);
    let snapshot = storage.open_snapshot_read();
    let prefix: StorageKey<'_, 48> =
        StorageKey::Reference(StorageKeyReference::new(keyspace, ByteReference::new(&[0_u8])));
    let iterator = snapshot.iterate_range(PrefixRange::new_within(prefix));
    let count = iterator.collect_cloned_vec(|_, _| ((), ())).unwrap().len();
    println!("Keys confirmed to be written: {}", count);
    count
}

fn bench_snapshot_read_get(
    storage: &MVCCStorage<WAL>,
    keyspace: TestKeyspaceSet,
) -> Option<ByteArray<BUFFER_VALUE_INLINE>> {
    let snapshot = storage.open_snapshot_read();
    let mut last: Option<ByteArray<BUFFER_VALUE_INLINE>> = None;
    for _ in 0..1 {
        last = snapshot.get(StorageKey::Array(random_key_24(keyspace)).as_reference()).unwrap();
    }
    last
}

fn bench_snapshot_read_iterate<const ITERATE_COUNT: usize>(
    storage: &MVCCStorage<WAL>,
    keyspace: TestKeyspaceSet,
) -> Option<ByteArray<BUFFER_VALUE_INLINE>> {
    let snapshot = storage.open_snapshot_read();
    let mut last: Option<ByteArray<BUFFER_VALUE_INLINE>> = None;
    for _ in 0..ITERATE_COUNT {
        last = snapshot.get(StorageKey::Array(random_key_4(keyspace)).as_reference()).unwrap();
    }
    last
}

fn bench_snapshot_write_put(storage: &MVCCStorage<WAL>, keyspace: TestKeyspaceSet, batch_size: usize) {
    let snapshot = storage.open_snapshot_write();
    for _ in 0..batch_size {
        snapshot.put(random_key_24(keyspace)).unwrap();
    }
    snapshot.commit().unwrap()
}

fn setup_storage(storage_path: &Path, key_count: usize) -> MVCCStorage<WAL> {
    let storage = MVCCStorage::recover::<TestKeyspaceSet>("storage_bench", storage_path).unwrap();
    let keys = populate_storage(&storage, Keyspace, key_count);
    println!("Initialised storage with '{}' keys", keys);
    storage
}

fn criterion_benchmark(c: &mut Criterion) {
    init_logging();
    const INITIAL_KEY_COUNT: usize = 10_000; // 10 million = approximately 0.2 GB of keys
    println!("In cirterion benchmark");
    {
        let storage_path = create_tmp_dir();
        let storage = setup_storage(&storage_path, INITIAL_KEY_COUNT);
        c.bench_function("snapshot_read_get", |b| b.iter(|| bench_snapshot_read_get(&storage, Keyspace)));
    }
    // {
    //     let storage_path = create_tmp_dir();
    //     let storage = setup_storage(&storage_path, INITIAL_KEY_COUNT);
    //     c.bench_function("snapshot_write_put", |b| b.iter(|| bench_snapshot_write_put(&storage, Keyspace, 100)));
    // }
    // {
    //     let storage_path = create_tmp_dir();
    //     let storage = setup_storage(&storage_path, INITIAL_KEY_COUNT);
    //     c.bench_function("snapshot_read_iterate", |b| b.iter(|| bench_snapshot_read_iterate::<1>(&storage, Keyspace)));
    // }
}
// --- Code to generate flamegraphs copied from https://www.jibbow.com/posts/criterion-flamegraphs/ ---
// This causes a SIGBUS on (mac) arm64 if the frequency is set too high.

pub struct FlamegraphProfiler<'a> {
    frequency: c_int,
    active_profiler: Option<ProfilerGuard<'a>>,
}

impl<'a> FlamegraphProfiler<'a> {
    #[allow(dead_code)]
    pub fn new(frequency: c_int) -> Self {
        FlamegraphProfiler { frequency, active_profiler: None }
    }
}

impl<'a> Profiler for FlamegraphProfiler<'a> {
    fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
        self.active_profiler = Some(ProfilerGuard::new(self.frequency).unwrap());
    }

    fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
        std::fs::create_dir_all(benchmark_dir).unwrap();
        let flamegraph_path = benchmark_dir.join("flamegraph.svg");
        let flamegraph_file = File::create(flamegraph_path).expect("File system error while creating flamegraph.svg");
        if let Some(profiler) = self.active_profiler.take() {
            profiler.report().build().unwrap().flamegraph(flamegraph_file).expect("Error writing flamegraph");
        }
    }
}

fn profiled() -> Criterion {
    Criterion::default().with_profiler(FlamegraphProfiler::new(100))
}

criterion_group!(
    name = benches;
    config = profiled();
    targets = criterion_benchmark
);

criterion_main!(benches);