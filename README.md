# Contract Integration Challenge
In this task, you will be using anvil (as a ETH backend) and REVM to do storage manipulations in order to get the 
smart contract to return a true value. You will need to make sure that you have foundry setup on your computer.
If you don't, go [here](https://book.getfoundry.sh/getting-started/installation) and install via your favourite method.

### Submission
1) Create a git repo with the solution and share it with our githubs: @jnoorchashm37 and @Will-Smith11
2) Send us an email with the subject line `Rust Interview - [Name]` at joseph@sorellalabs.xyz and will@sorellalabs.xyz

## Scope
### Setup
The environment given to you will have some problems while building. This is on purpose and is expected that you are able to fix and
get the rust environment working.

**Setup Requirments**
It is required that for our contract setup that we make sure that we always auto generate the bindings to our smart contracts.
This must be done with a Rust buildscript. Whenever there is a change, we want our buildscript to detect it and rebuild the contract bindings 
so that if there is any integration changes that would be breaking, it would be caught by tests.
The solution should have it auto-generate into a file, looking something along the lines of:
```rust
#[rustfmt::skip]
pub mod gate_lock {
    alloy::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq,Hash, serde::Serialize, serde::Deserialize)]
        GateLock,
        "path/to/GateLock.json"
    );
}
```

### Constraints 
1) No setup code can be modified. The only place you can add your code is in the `solve` function, and subsequent helper functions.
2) Must use REVM, in the solution.

### Goals
1) get codebase to compile by setting up a build script to automatically setup contract bindings
2) setup and adjust a REVM environment so we can bypass the locks that are set in the contract but are immutable by default.

### Solution
The solution should be able to run as many times as we want given the random payloads and always have the contract function `isSolved` return true.
Your solution should go in /bin/solution.rs
