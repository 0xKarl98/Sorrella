use std::fmt::Debug;

use alloy::{
    primitives::{keccak256, Address, U256},
    sol_types::{SolCall, SolValue}
};
use evm_knowledge::{
    contract_bindings::gate_lock::GateLock,
    environment_deployment::{deploy_lock_contract, spin_up_anvil_instance},
    fetch_values
};
use revm::{
    db::CacheDB,
    primitives::{EnvWithHandlerCfg, TxKind},
    DatabaseRef
};

/// To execute, add to bin
#[tokio::main]
async fn main() -> eyre::Result<()> {
    let controls = spin_up_anvil_instance().await?;
    let payload = fetch_values();
    let deploy_address = deploy_lock_contract(&controls, payload).await?;

    assert!(solve(deploy_address, controls).await?);
    Ok(())
}

const SLOT_OFFSET: u64 = 2;

// your solution goes here.
async fn solve<DB: DatabaseRef>(contract_address: Address, db: DB) -> eyre::Result<bool>
where
    DB: Send + Sync,
    <DB as DatabaseRef>::Error: Send + Sync + Debug
{
    let mut cache_db = CacheDB::new(db);

    let mut first_value = U256::ZERO;
    let mut second_value = U256::ZERO;
    let mut offsets = Vec::new();

    loop {
        // works cuz first slot always zero
        let slot = next_slot(first_value, second_value);
        // if first % 2 == 0 then we use it for  next slot
        let index = if first_value.checked_rem(U256::from(2)).unwrap().is_zero() {
            first_value
        } else {
            second_value
        };

        let data = cache_db.storage_ref(contract_address, slot).unwrap();

        if data.is_zero() {
            break;
        }
        offsets.push(index);

        let (f, s, _) = extract_values(data);
        first_value = f;
        second_value = s;

        let o = modify_to_true(data);

        cache_db
            .insert_account_storage(contract_address, slot, o)
            .expect("shouldn't fail");
        // override
    }

    let evm_handler = EnvWithHandlerCfg::default();
    let mut evm = revm::Evm::builder()
        .with_ref_db(cache_db)
        .with_env_with_handler_cfg(evm_handler)
        .modify_env(|env| {
            env.cfg.disable_balance_check = true;
            env.cfg.chain_id = 1;
        })
        .modify_tx_env(|tx| {
            tx.transact_to = TxKind::Call(contract_address);
            tx.data = GateLock::isSolvedCall::new((offsets,)).abi_encode().into();
        })
        .build();

    let result = evm.transact().unwrap();
    println!("{:?}", result);

    Ok(GateLock::isSolvedCall::abi_decode_returns(result.result.output().unwrap(), true)
        .unwrap()
        .res)
}

fn next_slot(first_value: U256, second_value: U256) -> U256 {
    let index = if first_value.checked_rem(U256::from(2)).unwrap().is_zero() {
        first_value
    } else {
        second_value
    };
    keccak256((index, SLOT_OFFSET).abi_encode()).into()
}

fn extract_values(data: U256) -> (U256, U256, bool) {
    let first_value = data & (U256::MAX >> U256::from(192));
    let second_value = (data >> U256::from(64)) & (U256::MAX >> 64);
    let lock = (data >> U256::from(224)) != U256::ZERO;

    (first_value, second_value, lock)
}

fn modify_to_true(data: U256) -> U256 {
    // we want to flip the 225 bit
    let val = U256::from(1) << 224;
    data + val
}
