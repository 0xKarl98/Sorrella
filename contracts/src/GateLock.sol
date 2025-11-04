// SPDX-License-Identifier: MIT
pragma solidity =0.8.26;

contract GateLock {
    // the layout of what we will compress
    struct Values {
        uint64 firstValue;
        uint160 secondValue;
        bool is_unlocked;
    }

    struct Payload {
        uint64 firstValue;
        uint160 secondValue;
    }

    error invalidLength();

    mapping(uint id => uint64 random) internal _a;
    mapping(address id => uint56 random) internal _b;
    mapping(uint id => Values values) internal valueMap;
    mapping(bytes32 id => uint128 random) internal _c;
    uint internal totalLength;

    //give initialPayload , slot is changing according to this condition : 
    //cur.firstValue % 2 == 0 
    //Suppose we have initPayload = [{100, 0x123}, {201, 0x456}, {88, 0x789}];  
    constructor(Payload[] memory initPayload) {
        uint length = initPayload.length;
        totalLength = length;

        uint slot = 0;

        //loop 3 times
        for (uint i = 0; i < length; i++) {
            Payload memory cur = initPayload[i];
            Values memory s = Values(cur.firstValue, cur.secondValue, false);
            
            //1. valueMap[0] = {100, 0x123, false} 
            //2. valueMap[100] = {201, 0x456, false} 
            //3. valueMap[0x456] = {88, 0x789, false} 
            valueMap[slot] = s;
            
            //0. slot == 0 (initial case )
            //1. slot == 100 , cur.firstValue == 100 
            //2. slot == 201 , cur.secondValue == 0x456
            //3. slot == 88  , cur.firstValue  == 88
            if (cur.firstValue % 2 == 0) {
                slot = cur.firstValue;
            } else {
                slot = cur.secondValue;
            }
        }
    }


    //ids must contain all the keys in constructor functions 
    //And all corresponding is_unlocked must be true 
    function isSolved(uint[] calldata ids) public view returns (bool res) {
        res = true;

        uint length = ids.length;

        if (length != totalLength) {
            revert invalidLength();
        }

        for (uint i = 0; i < length; i++) {
            res = res && valueMap[ids[i]].is_unlocked;
        }

        return res;
    }
}
