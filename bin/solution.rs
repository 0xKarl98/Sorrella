use alloy::primitives::{Address, B256, Bytes, U256, keccak256};
use alloy::sol_types::SolCall;
use evm_knowledge::{contract_bindings::gate_lock::GateLock, deploy_setup_with_solver};
use revm::{
    Database, DatabaseRef, Evm,
    primitives::{AccountInfo, Bytecode, ExecutionResult, Output, TransactTo},
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    deploy_setup_with_solver(solve).await?;
    Ok(())
}

// REVM storage manipulation solution
async fn solve<DB: DatabaseRef>(contract_address: Address, db: DB) -> eyre::Result<bool> {
    println!("Starting solution for contract at: {:?}", contract_address);

    let mut writable_db = WritableDatabase::new(db);

    let total_length_slot = U256::from(4);
    let value_map_slot = U256::from(2);

    // Read totalLength from storage
    let total_length_u256 = writable_db.storage_ref(contract_address, total_length_slot)?;
    let total_length: usize = total_length_u256
        .try_into()
        .map_err(|_| eyre::eyre!("totalLength too large"))?;
    println!("Total length: {}", total_length);

    if total_length == 0 {
        println!("Total length is 0, nothing to solve.");
        let call_result = call_is_solved_via_revm(&writable_db, contract_address, Vec::new())?;
        println!("isSolved result (length 0): {}", call_result);
        return Ok(call_result);
    }

    // ===================================================================
    // Simulate the execution process of the constructor and collect the actually written slot keys
    // ===================================================================
    let mut slot_data = std::collections::HashMap::new();
    let mut used_slots = Vec::new();
    let mut current_slot = U256::ZERO; // Constructor starts from slot 0

    println!("Simulating construction process...");

    for _i in 0..total_length {
        println!("The slot key to be written : {}", current_slot);

        // Compute the actual storage slot for the mapping
        let storage_slot = calculate_mapping_slot(current_slot, value_map_slot);

        // Read struct Values 
        let current_value = writable_db.storage_ref(contract_address, storage_slot)?;
        println!("Read Value (storage slot {}): 0x{:x}", storage_slot, current_value);

        // Check if the slot is empty
        if current_value == U256::ZERO {
            return Err(eyre::eyre!("slot {} is empty, constructor should have written data", current_slot));
        }

        // Analysis Values struct layout
        // struct Values {
        //     uint64 firstValue;    // 8 bytes, bits 0-63
        //     uint160 secondValue;  // 20 bytes, bits 64-223
        //     bool is_unlocked;     // 1 byte, bits 224-231
        // }
        println!("Current slot at {}", current_slot);
        println!("Current_value: 0x{:064x}", current_value);

        let first_value_mask = (U256::from(1) << 64) - U256::from(1);
        let first_value_u256 = current_value & first_value_mask;
        let first_value =
            u64::try_from(first_value_u256).map_err(|_| eyre::eyre!("firstValue overflow"))?;
        println!("convert to u64: {}", first_value);


        let second_value_mask = (U256::from(1) << 160) - U256::from(1);
        let shifted_value = current_value >> 64;
        let second_value_u160 = shifted_value & second_value_mask;
        println!("    - (current_value >> 64) & mask: 0x{:x}", second_value_u160);

        let is_unlocked_bit = (current_value >> 224) & U256::from(1);
        let is_unlocked = is_unlocked_bit != U256::ZERO;
        println!("    - is_unlocked: {}", is_unlocked);

        println!(
            "Final Result - firstValue: {}, secondValue: 0x{:x}, is_unlocked: {}",
            first_value, second_value_u160, is_unlocked
        );

        slot_data.insert(current_slot, (first_value, second_value_u160));
        used_slots.push(current_slot); 


        println!("=== Jump logic debugging ===");
        println!("firstValue: {}", first_value);
        println!("firstValue % 2: {}", first_value % 2);
        println!("secondValue: {}", second_value_u160);

        let next_slot = if first_value % 2 == 0 {
            println!("If firstValue % 2 == 0 is true, select firstValue.");
            U256::from(first_value)
        } else {
            println!("If firstValue % 2 == 0 is false, select secondValue.");
            second_value_u160
        };

        println!("Next slot key: {} (0x{:x})", next_slot, next_slot);

        // Check if it will cause an infinite loop.
        if next_slot == current_slot {
            println!("Warning: If the next slot is the same as the current slot, it may cause an infinite loop.");
        }

        current_slot = next_slot;
    }

    // Check if the amount of slot mathes totalLength 
    assert_eq!(
        used_slots.len(),
        total_length,
        "Collect {} slot,but totalLength is {}",
        used_slots.len(),
        total_length
    );

    if used_slots.len() != total_length {
        println!(
            "Warning: Collected {} slots, but totalLength is {}. The chain might have broken early.",
            used_slots.len(),
            total_length
        );
    }

    // ===================================================================
    // Write modified values to storage
    // ===================================================================
    println!("\nStarting storage manuplation...");

    for (&slot_key, &(first_value, second_value_u160)) in &slot_data {
        println!("Setting is_unlocked=true for slot key 0x{:x}", slot_key);

        // Calculate actual storage slot for the mapping
        let storage_slot = calculate_mapping_slot(slot_key, value_map_slot);

        let mut new_value = U256::ZERO;

        // Set firstValue (bits 0-63)
        new_value |= U256::from(first_value);

        // Set secondValue (bits 64-223)
        new_value |= second_value_u160 << 64;

        // Set is_unlocked to true (bits 224-231)
        new_value |= U256::from(1) << 224;

        // Wirte manuplated value to writable_db
        writable_db.set_storage(contract_address, storage_slot, new_value)?;
        println!("Wrote new value 0x{:x} to storage slot {}", new_value, storage_slot);
    }

    println!("Used slot keys (in order): {:?}", used_slots);
    println!("Total length: {}, Used slots length: {}", total_length, used_slots.len());

    // ===================================================================
    // Verify using isSolved 
    // ===================================================================

    println!("\n Verifying storage modifications before isSolved call:");
    for (&slot_key, _) in &slot_data {
        let storage_slot = calculate_mapping_slot(slot_key, value_map_slot);
        let current_value = writable_db.storage_ref(contract_address, storage_slot)?;
        let is_unlocked = (current_value & U256::from(1)) != U256::ZERO;
        println!(
            "Slot key 0x{:x} -> storage slot {}: is_unlocked = {}",
            slot_key, storage_slot, is_unlocked
        );
    }

    let ids: Vec<U256> = used_slots; 
    println!("Calling isSolved with {} ids", ids.len());
    println!("First few ids: {:?}", &ids[..std::cmp::min(10, ids.len())]);

    // Verify totalLength in contract 
    let contract_total_length = writable_db.storage_ref(contract_address, total_length_slot)?;
    println!("Contract totalLength: {}", contract_total_length);
    println!("Our ids length: {}", ids.len());

    let call_result = call_is_solved_via_revm(&writable_db, contract_address, ids)?;

    println!("IsSolved result: {}", call_result);
    Ok(call_result)
}

// Helper: Calculate mapping storage slot using Solidity's mapping storage layout
fn calculate_mapping_slot(key: U256, mapping_slot: U256) -> U256 {
    let mut data = [0u8; 64];

    // Encode key (left-padded to 32 bytes)
    let key_bytes = key.to_be_bytes::<32>();
    data[0..32].copy_from_slice(&key_bytes);

    // Encode mapping slot (left-padded to 32 bytes)
    let slot_bytes = mapping_slot.to_be_bytes::<32>();
    data[32..64].copy_from_slice(&slot_bytes);

    U256::from_be_slice(keccak256(data).as_slice())
}

// Helper: Call isSolved function via REVM
fn call_is_solved_via_revm<DB: DatabaseRef>(
    db: &WritableDatabase<DB>,
    contract_address: Address,
    ids: Vec<U256>,
) -> eyre::Result<bool> {

    // Manually check each id's is_unlocked status before isSolved call
    println!("Manual verification of all ids before isSolved:");
    let value_map_slot = U256::from(2);
    for (i, &id) in ids.iter().enumerate() {
        let storage_slot = calculate_mapping_slot(id, value_map_slot);
        let current_value = db.storage_ref(contract_address, storage_slot)?;
        let is_unlocked = (current_value & U256::from(1)) != U256::ZERO;
        if !is_unlocked {
            println!("ID {} (index {}): 0x{:x} has is_unlocked = false", i, i, id);
        } else {
            println!("ID {} (index {}): 0x{:x} has is_unlocked = true", i, i, id);
        }
        if i >= 10 && i < ids.len() - 10 {
            if i == 11 {
                println!("... (skipping middle entries for brevity) ...");
            }
            continue;
        }
    }

    // Encode isSolved call data
    let call_data = GateLock::isSolvedCall { ids }.abi_encode();

    // Create REVM instance
    let mut evm = Evm::builder()
        .with_db(db)
        .modify_tx_env(|tx| {
            tx.transact_to = TransactTo::Call(contract_address);
            tx.data = Bytes::from(call_data);
            tx.gas_limit = 1_000_000;
        })
        .build();

    // Execute the call
    let result = evm
        .transact()
        .map_err(|e| eyre::eyre!("REVM execution failed: {:?}", e))?;

    match result.result {
        ExecutionResult::Success { output: Output::Call(data), .. } => {
            println!("isSolved call succeeded, return data: {:?}", data);
            println!("Return data length: {}", data.len());
            // Parse boolean return value (32 bytes, last byte contains the boolean)
            if data.len() >= 32 {
                let result_bool = data[31] != 0;
                println!("Parsed result: {}", result_bool);
                println!("Last byte value: {}", data[31]);
                Ok(result_bool)
            } else {
                println!("Return data too short");
                Ok(false)
            }
        }
        ExecutionResult::Revert { output, .. } => {
            println!("Contract call reverted with data: {:?}", output);
            Ok(false)
        }
        ExecutionResult::Halt { reason, .. } => {
            println!("Contract call halted with reason: {:?}", reason);
            Ok(false)
        }
        _ => {
            println!("Unexpected execution result");
            Ok(false)
        }
    }
}

// Writable database wrapper that tracks storage changes
struct WritableDatabase<DB> {
    inner: DB,
    storage_changes: HashMap<(Address, U256), U256>,
}

impl<DB: DatabaseRef> WritableDatabase<DB> {
    fn new(inner: DB) -> Self {
        Self { inner, storage_changes: HashMap::new() }
    }

    fn set_storage(&mut self, address: Address, index: U256, value: U256) -> eyre::Result<()> {
        self.storage_changes.insert((address, index), value);
        Ok(())
    }
}

impl<DB: DatabaseRef> Database for &WritableDatabase<DB> {
    type Error = eyre::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.inner
            .basic_ref(address)
            .map_err(|_| eyre::eyre!("Database error"))
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.inner
            .code_by_hash_ref(code_hash)
            .map_err(|_| eyre::eyre!("Database error"))
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        if let Some(&value) = self.storage_changes.get(&(address, index)) {
            println!(
                "REVM reading MODIFIED storage: address=0x{:x}, index={}, value=0x{:x}",
                address, index, value
            );
            println!("REVM MODIFIED - is_unlocked: {}", (value & U256::from(1)) != U256::ZERO);
            Ok(value)
        } else {
            let value = self
                .inner
                .storage_ref(address, index)
                .map_err(|_| eyre::eyre!("Database error"))?;
            Ok(value)
        }
    }

    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        self.inner
            .block_hash_ref(number)
            .map_err(|_| eyre::eyre!("Database error"))
    }
}

impl<DB: DatabaseRef> DatabaseRef for WritableDatabase<DB> {
    type Error = eyre::Error;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.inner
            .basic_ref(address)
            .map_err(|_| eyre::eyre!("Database error"))
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.inner
            .code_by_hash_ref(code_hash)
            .map_err(|_| eyre::eyre!("Database error"))
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        if let Some(&value) = self.storage_changes.get(&(address, index)) {
            Ok(value)
        } else {
            self.inner
                .storage_ref(address, index)
                .map_err(|_| eyre::eyre!("Database error"))
        }
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        self.inner
            .block_hash_ref(number)
            .map_err(|_| eyre::eyre!("Database error"))
    }
}
