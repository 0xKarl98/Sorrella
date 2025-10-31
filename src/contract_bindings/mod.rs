#[rustfmt::skip]
pub mod gate_lock {
    alloy::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        GateLock,
        "contracts/out/GateLock.sol/GateLock.json"
    );
}
