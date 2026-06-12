// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test, console} from "forge-std/Test.sol";
import {ClaimRegistry} from "../src/ClaimRegistry.sol";
import {SettlementVault} from "../src/SettlementVault.sol";
import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {RiscZeroMockVerifier} from "risc0-ethereum/test/RiscZeroMockVerifier.sol";
import {AgentIdentityRegistry} from "../src/AgentIdentityRegistry.sol";

contract MockUSDT is ERC20 {
    constructor() ERC20("Mock USDT", "USDT") {
        _mint(msg.sender, 1000000 * 10 ** decimals());
    }
}

contract ClaimRegistryTest is Test {
    ClaimRegistry public registry;
    SettlementVault public vault;
    MockUSDT public token;
    RiscZeroMockVerifier public mockVerifier;
    AgentIdentityRegistry public identityRegistry;

    bytes32 public constant IMAGE_ID = bytes32(uint256(0x1234));
    bytes32 public constant ORACLE_KEY_HASH = keccak256("kedge-oracle");
    bytes32 public constant POLICY_ID = keccak256("kedge-policy");
    bytes32 public constant PAYLOAD_HASH = keccak256("oracle-payload");
    uint64 public constant CHAIN_ID = 5003;
    address public claimant = address(0xABCD);

    bytes4 public constant SELECTOR = bytes4(0x12345678);

    function setUp() public {
        // Deploy MockUSDT
        token = new MockUSDT();

        // Deploy RISC Zero mock verifier
        mockVerifier = new RiscZeroMockVerifier(SELECTOR);

        // Deploy Agent Identity Registry
        identityRegistry = new AgentIdentityRegistry();

        // Mint Identity to `address(this)` so the test can submit claims
        identityRegistry.mintIdentity(address(this), "ipfs://mock_metadata");

        // Deploy Registry
        registry = new ClaimRegistry(
            IRiscZeroVerifier(address(mockVerifier)), IMAGE_ID, address(identityRegistry), ORACLE_KEY_HASH, CHAIN_ID
        );

        // Deploy Vault
        vault = new SettlementVault(token, address(registry));

        // Connect Registry to Vault
        registry.setVault(address(vault));

        // Fund Vault
        token.transfer(address(vault), 100000 * 10 ** token.decimals());
    }

    function test_submitClaim_success() public {
        // Create the ClaimOutput struct
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256("TEST-001"),
            timestamp: uint64(block.timestamp),
            claimant: claimant,
            policyId: POLICY_ID,
            oracleKeyHash: ORACLE_KEY_HASH,
            payloadHash: PAYLOAD_HASH,
            expiresAt: uint64(block.timestamp + 5 minutes),
            chainId: CHAIN_ID
        });

        // ABI encode the journal
        bytes memory journal = abi.encode(output);

        // Mock the seal using the mock verifier
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        // Record initial balance
        uint256 initialBalance = token.balanceOf(claimant);

        // Submit claim
        registry.submitClaim(seal, journal);

        // Verify state
        assertTrue(registry.isClaimProcessed(output.payloadHash));
        assertEq(token.balanceOf(claimant), initialBalance + 25000);
    }

    function test_submitClaim_alreadyProcessed() public {
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256("TEST-002"),
            timestamp: uint64(block.timestamp),
            claimant: claimant,
            policyId: POLICY_ID,
            oracleKeyHash: ORACLE_KEY_HASH,
            payloadHash: PAYLOAD_HASH,
            expiresAt: uint64(block.timestamp + 5 minutes),
            chainId: CHAIN_ID
        });
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        // Submit first time
        registry.submitClaim(seal, journal);

        vm.expectRevert(
            abi.encodeWithSelector(ClaimRegistry.ClaimRegistry__ClaimAlreadyProcessed.selector, output.payloadHash)
        );
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_notTriggered() public {
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: false,
            payoutAmount: 0,
            payoutPercentage: 0,
            trackingIdHash: keccak256("TEST-003"),
            timestamp: uint64(block.timestamp),
            claimant: claimant,
            policyId: POLICY_ID,
            oracleKeyHash: ORACLE_KEY_HASH,
            payloadHash: PAYLOAD_HASH,
            expiresAt: uint64(block.timestamp + 5 minutes),
            chainId: CHAIN_ID
        });
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__ClaimNotTriggered.selector);
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_unauthorized() public {
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256("TEST-004"),
            timestamp: uint64(block.timestamp),
            claimant: claimant,
            policyId: POLICY_ID,
            oracleKeyHash: ORACLE_KEY_HASH,
            payloadHash: PAYLOAD_HASH,
            expiresAt: uint64(block.timestamp + 5 minutes),
            chainId: CHAIN_ID
        });
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        // Impersonate an unauthorized address
        address unauthorized = address(0xDEADBEEF);
        vm.prank(unauthorized);

        // Should revert with custom error
        vm.expectRevert(ClaimRegistry.ClaimRegistry__UnauthorizedAgent.selector);
        registry.submitClaim(seal, journal);
    }

    function test_setVault_unauthorized() public {
        ClaimRegistry newRegistry = new ClaimRegistry(
            IRiscZeroVerifier(address(mockVerifier)), IMAGE_ID, address(identityRegistry), ORACLE_KEY_HASH, CHAIN_ID
        );

        vm.prank(address(0xBAD));
        vm.expectRevert(abi.encodeWithSignature("OwnableUnauthorizedAccount(address)", address(0xBAD)));
        newRegistry.setVault(address(vault));
    }

    function test_setVault_cannotBeReconfigured() public {
        vm.expectRevert(ClaimRegistry.ClaimRegistry__VaultAlreadySet.selector);
        registry.setVault(address(vault));
    }

    function test_submitClaim_stale() public {
        vm.warp(2 days);

        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256("TEST-STALE"),
            timestamp: 1,
            claimant: claimant,
            policyId: POLICY_ID,
            oracleKeyHash: ORACLE_KEY_HASH,
            payloadHash: PAYLOAD_HASH,
            expiresAt: uint64(block.timestamp + 1 days),
            chainId: CHAIN_ID
        });
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__StaleClaim.selector);
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_zeroClaimant() public {
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256("TEST-ZERO-CLAIMANT"),
            timestamp: uint64(block.timestamp),
            claimant: address(0),
            policyId: POLICY_ID,
            oracleKeyHash: ORACLE_KEY_HASH,
            payloadHash: PAYLOAD_HASH,
            expiresAt: uint64(block.timestamp + 5 minutes),
            chainId: CHAIN_ID
        });
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__InvalidClaimant.selector);
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_zeroPolicyId() public {
        ClaimRegistry.ClaimOutput memory output = validOutput("TEST-ZERO-POLICY");
        output.policyId = bytes32(0);
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__InvalidPolicy.selector);
        registry.submitClaim(seal, journal);
    }

    function test_distinctPoliciesForShipmentHaveDistinctClaimIds() public {
        ClaimRegistry.ClaimOutput memory first = validOutput("TEST-MULTI-POLICY");
        first.payloadHash = keccak256("policy-a-payload");
        bytes memory firstJournal = abi.encode(first);
        registry.submitClaim(mockVerifier.mockProve(IMAGE_ID, sha256(firstJournal)).seal, firstJournal);

        ClaimRegistry.ClaimOutput memory second = validOutput("TEST-MULTI-POLICY");
        second.policyId = keccak256("policy-b");
        second.payloadHash = keccak256("policy-b-payload");
        bytes memory secondJournal = abi.encode(second);
        registry.submitClaim(mockVerifier.mockProve(IMAGE_ID, sha256(secondJournal)).seal, secondJournal);

        assertTrue(registry.isClaimProcessed(first.payloadHash));
        assertTrue(registry.isClaimProcessed(second.payloadHash));
        assertEq(token.balanceOf(claimant), first.payoutAmount + second.payoutAmount);
    }

    function test_submitClaim_rejectsUnauthorizedOracle() public {
        ClaimRegistry.ClaimOutput memory output = validOutput("TEST-BAD-ORACLE");
        output.oracleKeyHash = keccak256("attacker-oracle");
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__UnauthorizedOracle.selector);
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_rejectsWrongChain() public {
        ClaimRegistry.ClaimOutput memory output = validOutput("TEST-WRONG-CHAIN");
        output.chainId = 1;
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__WrongSettlementChain.selector);
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_rejectsExpiredPayload() public {
        vm.warp(10 minutes);
        ClaimRegistry.ClaimOutput memory output = validOutput("TEST-EXPIRED");
        output.timestamp = uint64(block.timestamp);
        output.expiresAt = uint64(block.timestamp - 1);
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__StaleClaim.selector);
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_rejectsMalformedJournal() public {
        bytes memory journal = hex"1234";
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(ClaimRegistry.ClaimRegistry__InvalidJournal.selector);
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_insufficientVaultFunds() public {
        MockUSDT emptyToken = new MockUSDT();
        ClaimRegistry emptyRegistry = new ClaimRegistry(
            IRiscZeroVerifier(address(mockVerifier)), IMAGE_ID, address(identityRegistry), ORACLE_KEY_HASH, CHAIN_ID
        );
        SettlementVault emptyVault = new SettlementVault(emptyToken, address(emptyRegistry));
        emptyRegistry.setVault(address(emptyVault));

        ClaimRegistry.ClaimOutput memory output = validOutput("TEST-EMPTY-VAULT");
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert(SettlementVault.InsufficientFunds.selector);
        emptyRegistry.submitClaim(seal, journal);
    }

    function test_identityTransferClearsAgentWallet() public {
        address newAgent = address(0xCAFE);
        uint256 agentId = identityRegistry.walletToAgentId(address(this));

        identityRegistry.transferFrom(address(this), newAgent, agentId);

        assertFalse(identityRegistry.isValidAgent(address(this)));
        assertFalse(identityRegistry.isValidAgent(newAgent));
        assertEq(identityRegistry.walletToAgentId(address(this)), 0);
        assertEq(identityRegistry.getAgentWallet(agentId), address(0));
    }

    function test_identityRegisterAndMetadata() public {
        address registrant = address(0xBEEF);
        vm.prank(registrant);
        uint256 agentId = identityRegistry.register("https://kedge.example/agent.json");

        assertEq(identityRegistry.ownerOf(agentId), registrant);
        assertEq(identityRegistry.getAgentWallet(agentId), registrant);
        assertTrue(identityRegistry.isValidAgent(registrant));

        vm.prank(registrant);
        identityRegistry.setMetadata(agentId, "proofSystem", bytes("risc0-groth16"));
        assertEq(identityRegistry.getMetadata(agentId, "proofSystem"), bytes("risc0-groth16"));
    }

    function test_identityWalletRequiresProofOfControl() public {
        uint256 newWalletKey = 0xA11CE;
        address newWallet = vm.addr(newWalletKey);
        uint256 agentId = identityRegistry.walletToAgentId(address(this));
        uint256 deadline = block.timestamp + 5 minutes;
        bytes32 digest = identityRegistry.agentWalletDigest(agentId, newWallet, deadline);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(newWalletKey, digest);

        identityRegistry.setAgentWallet(agentId, newWallet, deadline, abi.encodePacked(r, s, v));

        assertFalse(identityRegistry.isValidAgent(address(this)));
        assertTrue(identityRegistry.isValidAgent(newWallet));
        assertEq(identityRegistry.getAgentWallet(agentId), newWallet);
    }

    function test_identityRejectsReservedMetadataKey() public {
        uint256 agentId = identityRegistry.walletToAgentId(address(this));
        vm.expectRevert(AgentIdentityRegistry.AgentIdentityRegistry__ReservedMetadataKey.selector);
        identityRegistry.setMetadata(agentId, "agentWallet", abi.encode(address(0xBEEF)));
    }

    function validOutput(string memory trackingId) internal view returns (ClaimRegistry.ClaimOutput memory) {
        return ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256(bytes(trackingId)),
            timestamp: uint64(block.timestamp),
            claimant: claimant,
            policyId: POLICY_ID,
            oracleKeyHash: ORACLE_KEY_HASH,
            payloadHash: PAYLOAD_HASH,
            expiresAt: uint64(block.timestamp + 5 minutes),
            chainId: CHAIN_ID
        });
    }
}
