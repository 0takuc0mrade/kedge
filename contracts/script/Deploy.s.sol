// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console2} from "forge-std/Script.sol";
import {SafeERC20} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {ControlID, RiscZeroGroth16Verifier} from "risc0-ethereum/groth16/RiscZeroGroth16Verifier.sol";
import {RiscZeroMockVerifier} from "risc0-ethereum/test/RiscZeroMockVerifier.sol";
import {AgentIdentityRegistry} from "../src/AgentIdentityRegistry.sol";
import {ClaimRegistry} from "../src/ClaimRegistry.sol";
import {MockUSDT} from "../src/MockUSDT.sol";
import {SettlementVault} from "../src/SettlementVault.sol";

contract DeployMantleSepolia is Script {
    using SafeERC20 for IERC20;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address agentWallet = vm.envAddress("AGENT_WALLET_ADDRESS");
        bytes32 imageId = vm.envBytes32("CLAIM_EVALUATOR_IMAGE_ID");
        bytes32 oracleKeyHash = vm.envBytes32("ORACLE_KEY_HASH");
        uint64 chainId = uint64(vm.envUint("CHAIN_ID"));
        string memory agentMetadataURI = vm.envString("AGENT_METADATA_URI");
        uint256 vaultFunding = vm.envUint("VAULT_FUNDING_AMOUNT");

        require(chainId == block.chainid, "CHAIN_ID does not match the connected network");
        require(agentWallet != address(0), "AGENT_WALLET_ADDRESS is zero");

        vm.startBroadcast(deployerPrivateKey);

        RiscZeroGroth16Verifier verifier =
            new RiscZeroGroth16Verifier(ControlID.CONTROL_ROOT, ControlID.BN254_CONTROL_ID);
        AgentIdentityRegistry identityRegistry = new AgentIdentityRegistry();
        uint256 agentId = identityRegistry.mintIdentity(agentWallet, agentMetadataURI);
        MockUSDT token = new MockUSDT();
        ClaimRegistry registry = new ClaimRegistry(
            IRiscZeroVerifier(address(verifier)), imageId, address(identityRegistry), oracleKeyHash, chainId
        );
        SettlementVault vault = new SettlementVault(token, address(registry));

        registry.setVault(address(vault));
        IERC20(address(token)).safeTransfer(address(vault), vaultFunding);

        vm.stopBroadcast();

        console2.log("RiscZeroGroth16Verifier:", address(verifier));
        console2.log("AgentIdentityRegistry:", address(identityRegistry));
        console2.log("Agent ID:", agentId);
        console2.log("MockUSDT:", address(token));
        console2.log("ClaimRegistry:", address(registry));
        console2.log("SettlementVault:", address(vault));
        console2.log("Vault funding:", vaultFunding);
    }
}

contract DeployLocal is Script {
    using SafeERC20 for IERC20;

    bytes4 internal constant MOCK_SELECTOR = 0x12345678;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address agentWallet = vm.addr(deployerPrivateKey);
        bytes32 imageId = vm.envBytes32("CLAIM_EVALUATOR_IMAGE_ID");
        bytes32 oracleKeyHash = vm.envBytes32("ORACLE_KEY_HASH");
        uint64 chainId = uint64(block.chainid);

        vm.startBroadcast(deployerPrivateKey);

        RiscZeroMockVerifier verifier = new RiscZeroMockVerifier(MOCK_SELECTOR);
        AgentIdentityRegistry identityRegistry = new AgentIdentityRegistry();
        identityRegistry.mintIdentity(agentWallet, "ipfs://kedge-local-agent");
        MockUSDT token = new MockUSDT();
        ClaimRegistry registry = new ClaimRegistry(
            IRiscZeroVerifier(address(verifier)), imageId, address(identityRegistry), oracleKeyHash, chainId
        );
        SettlementVault vault = new SettlementVault(token, address(registry));

        registry.setVault(address(vault));
        IERC20(address(token)).safeTransfer(address(vault), 100_000 * 10 ** token.decimals());

        vm.stopBroadcast();

        console2.log("RiscZeroMockVerifier:", address(verifier));
        console2.log("AgentIdentityRegistry:", address(identityRegistry));
        console2.log("MockUSDT:", address(token));
        console2.log("ClaimRegistry:", address(registry));
        console2.log("SettlementVault:", address(vault));
    }
}
