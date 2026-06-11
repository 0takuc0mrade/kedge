// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC721URIStorage, ERC721} from "openzeppelin-contracts/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import {Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";

contract AgentIdentityRegistry is ERC721URIStorage, Ownable {
    uint256 private _nextTokenId;

    // ETH Skills: Custom errors for gas efficiency
    error AgentIdentityRegistry__AlreadyRegistered(address agentWallet);
    error AgentIdentityRegistry__NotAuthorized();

    mapping(address => uint256) public walletToAgentId;
    mapping(address => bool) public isRegistered;

    constructor() ERC721("Kedge Agent Identity", "KAI") Ownable(msg.sender) {
        _nextTokenId = 1;
    }

    /// @notice Mints an ERC-8004 compliant identity NFT to the agent's hot wallet
    function mintIdentity(address agentWallet, string memory metadataURI) external onlyOwner returns (uint256) {
        if (isRegistered[agentWallet]) revert AgentIdentityRegistry__AlreadyRegistered(agentWallet);

        uint256 tokenId = _nextTokenId++;
        isRegistered[agentWallet] = true;
        walletToAgentId[agentWallet] = tokenId;

        _mint(agentWallet, tokenId);
        _setTokenURI(tokenId, metadataURI);

        return tokenId;
    }

    /// @notice Verifies if a wallet holds an active agent identity
    function isValidAgent(address agentWallet) external view returns (bool) {
        return isRegistered[agentWallet];
    }
}
