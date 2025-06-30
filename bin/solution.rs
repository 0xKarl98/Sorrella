use alloy::primitives::Address;
use evm_knowledge::{
    deploy_setup_with_solver,
    environment_deployment::{deploy_lock_contract, spin_up_anvil_instance},
};
use revm::DatabaseRef;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    deploy_setup_with_solver(solve).await?;

    Ok(())
}

// your solution goes here.
async fn solve<DB: DatabaseRef>(contract_address: Address, db: DB) -> eyre::Result<bool> {
    Ok(false)
}
