use std::{env, fs, process::ExitCode};

use shared::wasm::{canonical_wasm_hash_v1, CANONICAL_WASM_HASH_V1};

fn main() -> ExitCode {
    let paths = env::args().skip(1).collect::<Vec<_>>();
    if paths.len() != 2 {
        eprintln!("usage: compare_canonical_wasm <first.wasm> <second.wasm>");
        return ExitCode::FAILURE;
    }

    let read =
        |path: &str| fs::read(path).map_err(|err| format!("failed to read artifact {path}: {err}"));
    let result = (|| {
        let first = read(&paths[0])?;
        let second = read(&paths[1])?;
        let first_hash = canonical_wasm_hash_v1(&first)
            .map_err(|err| format!("failed to canonicalize {}: {err}", paths[0]))?;
        let second_hash = canonical_wasm_hash_v1(&second)
            .map_err(|err| format!("failed to canonicalize {}: {err}", paths[1]))?;

        println!("algorithm={CANONICAL_WASM_HASH_V1}");
        println!("raw_equal={}", first == second);
        println!("first_canonical_hash={first_hash}");
        println!("second_canonical_hash={second_hash}");

        if first_hash != second_hash {
            return Err("independent builds differ inside the V1 trust boundary".to_string());
        }
        Ok(())
    })();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
