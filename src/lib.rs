pub mod contract_bindings;
pub mod environment_deployment;

use std::fmt::Debug;
use std::future::Future;

use alloy::primitives::{Address, U160, U256};
use environment_deployment::{AnvilControls, deploy_lock_contract, spin_up_anvil_instance};
use rand::{self, Rng};
use revm::DatabaseRef;

/// Payload structure matching the Solidity contract
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    pub firstValue: u64,
    pub secondValue: U160,
}

/// generates values for smart_contract
fn fetch_values() -> Vec<Payload> {
    let mut rng = rand::rng();
    let iter_cnt: usize = rng.random_range(10..100);

    (0..iter_cnt)
        .map(|_| {
            let bytes: [u8; 20] = rng.random();
            Payload { firstValue: rng.random(), secondValue: U160::from_be_bytes(bytes) }
        })
        .collect::<Vec<_>>()
}

pub async fn deploy_setup_with_solver<F, O>(f: F) -> eyre::Result<bool>
where
    F: Fn(Address, AnvilControls) -> O,
    O: Future<Output = eyre::Result<bool>>,
{
    let controls = spin_up_anvil_instance().await?;
    let payload = fetch_values();

    let deploy_address = deploy_lock_contract(&controls, payload).await?;

    f(deploy_address, controls).await
}
