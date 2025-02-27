#[rustfmt::skip]
pub mod gate_lock {
    alloy::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq,Hash, serde::Serialize, serde::Deserialize)]
        GateLock,
        "/Users/will/ghq/github.com/SorellaLabs/interview-questions/take-homes/evm-knowledge/contracts/out/GateLock.sol/GateLock.json"
    );
}
