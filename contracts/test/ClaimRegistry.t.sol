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
        registry = new ClaimRegistry(IRiscZeroVerifier(address(mockVerifier)), IMAGE_ID, address(identityRegistry));

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
            claimant: claimant
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
        assertTrue(registry.isClaimProcessed(output.trackingIdHash));
        assertEq(token.balanceOf(claimant), initialBalance + 25000);
    }

    function test_submitClaim_alreadyProcessed() public {
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256("TEST-002"),
            timestamp: uint64(block.timestamp),
            claimant: claimant
        });
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        // Submit first time
        registry.submitClaim(seal, journal);

        // Submit second time should fail
        vm.expectRevert("Claim already processed");
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_notTriggered() public {
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: false,
            payoutAmount: 0,
            payoutPercentage: 0,
            trackingIdHash: keccak256("TEST-003"),
            timestamp: uint64(block.timestamp),
            claimant: claimant
        });
        bytes memory journal = abi.encode(output);
        bytes memory seal = mockVerifier.mockProve(IMAGE_ID, sha256(journal)).seal;

        vm.expectRevert("Claim conditions not met");
        registry.submitClaim(seal, journal);
    }

    function test_submitClaim_unauthorized() public {
        ClaimRegistry.ClaimOutput memory output = ClaimRegistry.ClaimOutput({
            isTriggered: true,
            payoutAmount: 25000,
            payoutPercentage: 25,
            trackingIdHash: keccak256("TEST-004"),
            timestamp: uint64(block.timestamp),
            claimant: claimant
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
}
