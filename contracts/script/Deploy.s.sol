// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console} from "forge-std/Script.sol";
import {ClaimRegistry} from "../src/ClaimRegistry.sol";
import {SettlementVault} from "../src/SettlementVault.sol";
import {MockUSDT} from "../src/MockUSDT.sol";
import {AgentIdentityRegistry} from "../src/AgentIdentityRegistry.sol";
import {RiscZeroMockVerifier} from "risc0-ethereum/test/RiscZeroMockVerifier.sol";
import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";

contract DeployScript is Script {
    function run() external {
        // The first Anvil private key
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address agentWallet = vm.addr(deployerPrivateKey);

        // The image ID used by kedge-methods
        bytes32 imageId = 0x85a014194d3b315e4ea8c5236b00249882cca5564c9c9982ba11cc104331408e;
        bytes4 selector = bytes4(0x12345678);

        vm.startBroadcast(deployerPrivateKey);

        // 1. Deploy AgentIdentityRegistry and Mint NFT
        AgentIdentityRegistry identityRegistry = new AgentIdentityRegistry();
        console.log("AgentIdentityRegistry deployed at:", address(identityRegistry));

        string memory mockIpfsURI = "ipfs://QmYwAPJzv5CZsnA625s3Xf2bNx2RVX2pACuJb1Cq5pTq1"; // Mock IPFS hash
        uint256 tokenId = identityRegistry.mintIdentity(agentWallet, mockIpfsURI);
        console.log("Minted Identity NFT TokenID:", tokenId, "to", agentWallet);

        // 2. Deploy MockUSDT
        MockUSDT token = new MockUSDT();
        console.log("MockUSDT deployed at:", address(token));

        // 3. Deploy RiscZeroMockVerifier
        RiscZeroMockVerifier mockVerifier = new RiscZeroMockVerifier(selector);
        console.log("RiscZeroMockVerifier deployed at:", address(mockVerifier));

        // 4. Deploy ClaimRegistry (now requires identityRegistry)
        ClaimRegistry registry =
            new ClaimRegistry(IRiscZeroVerifier(address(mockVerifier)), imageId, address(identityRegistry));
        console.log("ClaimRegistry deployed at:", address(registry));

        // 4. Deploy SettlementVault
        SettlementVault vault = new SettlementVault(token, address(registry));
        console.log("SettlementVault deployed at:", address(vault));

        // 5. Connect Registry to Vault
        registry.setVault(address(vault));
        console.log("ClaimRegistry vault set to:", address(vault));

        // 6. Fund the Vault
        token.transfer(address(vault), 100000 * 10 ** token.decimals());
        console.log("Vault funded with 100,000 MockUSDT");

        vm.stopBroadcast();
    }
}
