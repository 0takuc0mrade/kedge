// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {AgentIdentityRegistry} from "./AgentIdentityRegistry.sol";

// Minimal interface for the vault
interface ISettlementVault {
    function disburse(address claimant, uint256 amount) external;
}

contract ClaimRegistry {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public immutable claimEvaluatorImageId;
    ISettlementVault public settlementVault;
    AgentIdentityRegistry public identityRegistry;

    // ETH Skills: Custom error
    error ClaimRegistry__UnauthorizedAgent();

    mapping(bytes32 => bool) public isClaimProcessed;

    // ALIGNED WITH ALLOY-SOL-TYPES IN RUST
    struct ClaimOutput {
        bool isTriggered;
        uint64 payoutAmount;
        uint64 payoutPercentage;
        bytes32 trackingIdHash;
        uint64 timestamp;
        address claimant; // CRITICAL FIX: Ensure funds route correctly
    }

    // Vault is set post-deployment or in constructor
    constructor(IRiscZeroVerifier _verifier, bytes32 _imageId, address _identityRegistry) {
        verifier = _verifier;
        claimEvaluatorImageId = _imageId;
        identityRegistry = AgentIdentityRegistry(_identityRegistry);
    }

    // New Modifier
    modifier onlyRegisteredAgent() {
        if (!identityRegistry.isValidAgent(msg.sender)) {
            revert ClaimRegistry__UnauthorizedAgent();
        }
        _;
    }

    function setVault(address _vault) external {
        // In production, add onlyOwner modifier here
        require(address(settlementVault) == address(0), "Vault already set");
        settlementVault = ISettlementVault(_vault);
    }

    function submitClaim(bytes calldata seal, bytes calldata journal) external onlyRegisteredAgent {
        // 1. Verify the cryptographic proof
        bytes32 journalDigest = sha256(journal);
        verifier.verify(seal, claimEvaluatorImageId, journalDigest);

        // 2. Decode the standard ABI-encoded journal
        ClaimOutput memory output = abi.decode(journal, (ClaimOutput));

        // 3. Evaluate state
        require(output.isTriggered, "Claim conditions not met");
        require(!isClaimProcessed[output.trackingIdHash], "Claim already processed");

        // 4. Lock state to prevent re-entrancy / double-spending
        isClaimProcessed[output.trackingIdHash] = true;

        // 5. Trigger the external treasury payout
        settlementVault.disburse(output.claimant, output.payoutAmount);
    }
}
