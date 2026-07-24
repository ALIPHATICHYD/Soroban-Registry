use soroban_registry_cli::profiler::{generate_flame_graph, FunctionProfile, ProfileData};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;
use sysinfo::{ProcessExt, System, SystemExt};
use tempfile::NamedTempFile;

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

fn main() {
    let profile = make_large_profile(50_000);
    let mut sys = System::new_all();
    sys.refresh_processes();
    let pid = sysinfo::get_current_pid().expect("pid");

    let before = sys.process(pid).map(|p| p.memory()).unwrap_or(0);
    println!("Memory before: {} KB", before);

    for i in 0..5 {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        generate_flame_graph(&profile, &path).unwrap();
        let _ = fs::read_to_string(&path).unwrap();
        sys.refresh_process(pid);
        let after = sys.process(pid).map(|p| p.memory()).unwrap_or(0);
        println!("Iteration {} memory: {} KB", i, after);
    }

    let sys_final = System::new_all();
    let final_mem = sys_final.process(pid).map(|p| p.memory()).unwrap_or(0);
    println!("Memory final: {} KB", final_mem);
}
