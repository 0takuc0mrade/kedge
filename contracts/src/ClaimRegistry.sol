// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {AgentIdentityRegistry} from "./AgentIdentityRegistry.sol";
import {Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";

// Minimal interface for the vault
interface ISettlementVault {
    function disburse(address claimant, uint256 amount) external;
}

contract ClaimRegistry is Ownable {
    uint256 public constant MAX_CLAIM_AGE = 1 days;

    IRiscZeroVerifier public immutable verifier;
    bytes32 public immutable claimEvaluatorImageId;
    bytes32 public immutable authorizedOracleKeyHash;
    uint64 public immutable settlementChainId;
    ISettlementVault public settlementVault;
    AgentIdentityRegistry public immutable identityRegistry;

    error ClaimRegistry__UnauthorizedAgent();
    error ClaimRegistry__ZeroAddress();
    error ClaimRegistry__VaultAlreadySet();
    error ClaimRegistry__VaultNotSet();
    error ClaimRegistry__ClaimNotTriggered();
    error ClaimRegistry__ClaimAlreadyProcessed(bytes32 claimId);
    error ClaimRegistry__InvalidClaimant();
    error ClaimRegistry__InvalidPolicy();
    error ClaimRegistry__InvalidPayout();
    error ClaimRegistry__StaleClaim();
    error ClaimRegistry__FutureClaim();
    error ClaimRegistry__UnauthorizedOracle();
    error ClaimRegistry__WrongSettlementChain();
    error ClaimRegistry__InvalidJournal();

    event VaultConfigured(address indexed vault);
    event ClaimSettled(
        bytes32 indexed claimId,
        bytes32 indexed policyId,
        address indexed claimant,
        address agent,
        bytes32 payloadHash,
        uint256 payoutAmount,
        uint256 payoutPercentage,
        uint256 observedAt
    );

    mapping(bytes32 => bool) public isClaimProcessed;

    struct ClaimOutput {
        bool isTriggered;
        uint64 payoutAmount;
        uint64 payoutPercentage;
        bytes32 trackingIdHash;
        uint64 timestamp;
        address claimant;
        bytes32 policyId;
        bytes32 oracleKeyHash;
        bytes32 payloadHash;
        uint64 expiresAt;
        uint64 chainId;
    }

    constructor(
        IRiscZeroVerifier _verifier,
        bytes32 _imageId,
        address _identityRegistry,
        bytes32 _authorizedOracleKeyHash,
        uint64 _settlementChainId
    ) Ownable(msg.sender) {
        if (
            address(_verifier) == address(0) || _identityRegistry == address(0)
                || _authorizedOracleKeyHash == bytes32(0) || _settlementChainId == 0
        ) {
            revert ClaimRegistry__ZeroAddress();
        }

        verifier = _verifier;
        claimEvaluatorImageId = _imageId;
        identityRegistry = AgentIdentityRegistry(_identityRegistry);
        authorizedOracleKeyHash = _authorizedOracleKeyHash;
        settlementChainId = _settlementChainId;
    }

    modifier onlyRegisteredAgent() {
        if (!identityRegistry.isValidAgent(msg.sender)) {
            revert ClaimRegistry__UnauthorizedAgent();
        }
        _;
    }

    function setVault(address _vault) external onlyOwner {
        if (_vault == address(0)) revert ClaimRegistry__ZeroAddress();
        if (address(settlementVault) != address(0)) revert ClaimRegistry__VaultAlreadySet();

        settlementVault = ISettlementVault(_vault);
        emit VaultConfigured(_vault);
    }

    function submitClaim(bytes calldata seal, bytes calldata journal) external onlyRegisteredAgent {
        if (address(settlementVault) == address(0)) revert ClaimRegistry__VaultNotSet();

        bytes32 journalDigest = sha256(journal);
        verifier.verify(seal, claimEvaluatorImageId, journalDigest);

        if (journal.length != 352) revert ClaimRegistry__InvalidJournal();
        ClaimOutput memory output = abi.decode(journal, (ClaimOutput));

        if (!output.isTriggered) revert ClaimRegistry__ClaimNotTriggered();
        if (output.claimant == address(0)) revert ClaimRegistry__InvalidClaimant();
        if (output.policyId == bytes32(0)) revert ClaimRegistry__InvalidPolicy();
        if (output.payoutAmount == 0 || output.payoutPercentage == 0 || output.payoutPercentage > 100) {
            revert ClaimRegistry__InvalidPayout();
        }
        if (output.oracleKeyHash != authorizedOracleKeyHash) revert ClaimRegistry__UnauthorizedOracle();
        if (output.chainId != settlementChainId) revert ClaimRegistry__WrongSettlementChain();
        if (output.timestamp > block.timestamp) revert ClaimRegistry__FutureClaim();
        if (block.timestamp - output.timestamp > MAX_CLAIM_AGE) revert ClaimRegistry__StaleClaim();
        if (output.expiresAt < block.timestamp) revert ClaimRegistry__StaleClaim();
        // payloadHash commits to policy, claimant, shipment, nonce, and settlement chain.
        bytes32 claimId = output.payloadHash;
        if (claimId == bytes32(0)) revert ClaimRegistry__InvalidJournal();
        if (isClaimProcessed[claimId]) {
            revert ClaimRegistry__ClaimAlreadyProcessed(claimId);
        }

        isClaimProcessed[claimId] = true;
        settlementVault.disburse(output.claimant, output.payoutAmount);

        emit ClaimSettled(
            claimId,
            output.policyId,
            output.claimant,
            msg.sender,
            output.payloadHash,
            output.payoutAmount,
            output.payoutPercentage,
            output.timestamp
        );
    }
}
