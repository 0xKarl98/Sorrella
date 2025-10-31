use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use convert_case::{Case, Casing};
use itertools::Itertools;

fn main() {
    println!("cargo:rerun-if-changed=contracts/src");
    println!("cargo:rerun-if-changed=contracts/foundry.toml");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let contracts_dir = Path::new(&manifest_dir).join("contracts");
    let src_dir = Path::new(&manifest_dir).join("src");
    let bindings_dir = src_dir.join("contract_bindings");

    // Ensure the contract bindings directory exists
    if !bindings_dir.exists() {
        fs::create_dir_all(&bindings_dir).expect("Failed to create contract_bindings directory");
    }

    // Run forge build to compile contracts
    compile_contracts(&contracts_dir);

    // Generate contract bindings
    generate_contract_bindings(&contracts_dir, &bindings_dir);
}

fn compile_contracts(contracts_dir: &Path) {
    println!("cargo:warning=Compiling Solidity contracts...");

    let output = Command::new("forge")
        .arg("build")
        .current_dir(contracts_dir)
        .output()
        .expect("Failed to execute forge build. Make sure foundry is installed.");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("forge build failed: {}", stderr);
    }

    println!("cargo:warning=Contracts compiled successfully");
}

fn generate_contract_bindings(contracts_dir: &Path, bindings_dir: &Path) {
    let out_dir = contracts_dir.join("out");

    if !out_dir.exists() {
        panic!("Contracts output directory not found. Make sure forge build was successful.");
    }

    let mut contract_modules = Vec::new();

    // Find all compiled contract JSON files recursively
    find_contract_files(&out_dir, &mut contract_modules, contracts_dir);

    if contract_modules.is_empty() {
        println!("cargo:warning=No contract files found in {}", out_dir.display());
        return;
    }

    // Generate the main mod.rs file
    let mod_content = contract_modules
        .iter()
        .map(|(_, module_content)| module_content.as_str())
        .join("\n\n");

    let mod_file_path = bindings_dir.join("mod.rs");
    fs::write(&mod_file_path, mod_content).expect("Failed to write contract bindings mod.rs");

    println!("cargo:warning=Contract bindings generated at: {}", mod_file_path.display());
}

fn find_contract_files(
    dir: &Path,
    contract_modules: &mut Vec<(String, String)>,
    contracts_dir: &Path,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                find_contract_files(&path, contract_modules, contracts_dir);
            } else if let Some(extension) = path.extension() {
                if extension == "json" {
                    if let Some(contract_name) = path.file_stem().and_then(|s| s.to_str()) {
                        // Skip debug files, test files, and build-info files
                        if contract_name.contains(".dbg")
                            || path.to_string_lossy().contains(".s.sol")
                            || path.to_string_lossy().contains("build-info")
                            || contract_name.chars().all(|c| c.is_ascii_hexdigit())
                        {
                            continue;
                        }

                        // Generate module name (snake_case) and ensure it starts with a letter
                        let mut module_name = contract_name.to_case(Case::Snake);

                        // If module name starts with a digit, prefix with "contract_"
                        if module_name
                            .chars()
                            .next()
                            .map_or(false, |c| c.is_ascii_digit())
                        {
                            module_name = format!("contract_{}", module_name);
                        }

                        // Get relative path from project root
                        let relative_path = path
                            .strip_prefix(contracts_dir.parent().unwrap())
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/");

                        let module_content = format!(
                            r#"#[rustfmt::skip]
pub mod {} {{
    alloy::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        {},
        "{}"
    );
}}"#,
                            module_name, contract_name, relative_path
                        );

                        contract_modules.push((module_name.clone(), module_content));

                        println!(
                            "cargo:warning=Generated bindings for contract: {}",
                            contract_name
                        );
                    }
                }
            }
        }
    }
}
