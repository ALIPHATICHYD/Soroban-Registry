use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;
use tempfile::NamedTempFile;

use soroban_registry_cli::profiler::{
    generate_flame_graph, generate_flame_graph_old, FunctionProfile, ProfileData,
};

fn make_large_profile(n: usize) -> ProfileData {
    let mut functions = HashMap::new();
    for i in 0..n {
        let name = format!("func_{}", i);
        let dur = Duration::from_nanos((i as u64 + 1) * 1_000_000);
        functions.insert(
            name.clone(),
            FunctionProfile {
                name,
                total_time: dur,
                call_count: (i as u64) + 1,
                avg_time: dur,
                min_time: dur,
                max_time: dur,
                children: vec![],
            },
        );
    }

    ProfileData {
        contract_path: "test".to_string(),
        method: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
        total_duration: Duration::from_secs(1),
        functions,
        call_stack: vec![],
        overhead_percent: 0.0,
    }
}

fn bench_old(c: &mut Criterion) {
    let profile = make_large_profile(2000);
    c.bench_function("generate_old", |b| {
        b.iter(|| {
            let tmp = NamedTempFile::new().unwrap();
            let path = tmp.path().to_path_buf();
            generate_flame_graph_old(&profile, &path).unwrap();
            let _ = fs::read_to_string(path).unwrap();
        })
    });
}

fn bench_builder(c: &mut Criterion) {
    let profile = make_large_profile(2000);
    c.bench_function("generate_builder", |b| {
        b.iter(|| {
            let tmp = NamedTempFile::new().unwrap();
            let path = tmp.path().to_path_buf();
            generate_flame_graph(&profile, &path).unwrap();
            let _ = fs::read_to_string(path).unwrap();
        })
    });
}

criterion_group!(benches, bench_old, bench_builder);
criterion_main!(benches);
