// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {ERC721URIStorage, ERC721} from "openzeppelin-contracts/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import {Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";
import {EIP712} from "openzeppelin-contracts/contracts/utils/cryptography/EIP712.sol";
import {ECDSA} from "openzeppelin-contracts/contracts/utils/cryptography/ECDSA.sol";
import {IERC1271} from "openzeppelin-contracts/contracts/interfaces/IERC1271.sol";

contract AgentIdentityRegistry is ERC721URIStorage, Ownable, EIP712 {
    struct MetadataEntry {
        string metadataKey;
        bytes metadataValue;
    }

    bytes4 private constant ERC1271_MAGIC_VALUE = IERC1271.isValidSignature.selector;
    bytes32 private constant AGENT_WALLET_KEY = keccak256("agentWallet");
    bytes32 private constant SET_AGENT_WALLET_TYPEHASH =
        keccak256("SetAgentWallet(uint256 agentId,address newWallet,uint256 deadline)");

    uint256 private _nextTokenId = 1;

    mapping(uint256 => mapping(bytes32 => bytes)) private _metadata;
    mapping(uint256 => address) private _agentWallets;
    mapping(address => uint256) public walletToAgentId;
    mapping(address => bool) public isRegistered;

    error AgentIdentityRegistry__AlreadyRegistered(address agentWallet);
    error AgentIdentityRegistry__ExpiredSignature();
    error AgentIdentityRegistry__InvalidSignature();
    error AgentIdentityRegistry__NotAuthorized();
    error AgentIdentityRegistry__ReservedMetadataKey();
    error AgentIdentityRegistry__ZeroAddress();

    event Registered(uint256 indexed agentId, string agentURI, address indexed owner);
    event URIUpdated(uint256 indexed agentId, string newURI, address indexed updatedBy);
    event MetadataSet(
        uint256 indexed agentId, string indexed indexedMetadataKey, string metadataKey, bytes metadataValue
    );

    constructor()
        ERC721("Kedge Agent Identity", "KAI")
        Ownable(msg.sender)
        EIP712("Kedge Agent Identity Registry", "1")
    {}

    function register() external returns (uint256 agentId) {
        MetadataEntry[] memory metadata = new MetadataEntry[](0);
        return _register(msg.sender, "", metadata);
    }

    function register(string calldata agentURI) external returns (uint256 agentId) {
        MetadataEntry[] memory metadata = new MetadataEntry[](0);
        return _register(msg.sender, agentURI, metadata);
    }

    function register(string calldata agentURI, MetadataEntry[] calldata metadata) external returns (uint256 agentId) {
        return _register(msg.sender, agentURI, metadata);
    }

    /// @notice Owner-assisted registration used by deployment scripts.
    function mintIdentity(address agentWallet, string memory agentURI) external onlyOwner returns (uint256 agentId) {
        if (agentWallet == address(0)) revert AgentIdentityRegistry__ZeroAddress();
        MetadataEntry[] memory metadata = new MetadataEntry[](0);
        return _register(agentWallet, agentURI, metadata);
    }

    function setAgentURI(uint256 agentId, string calldata newURI) external {
        _requireAgentAuthorization(agentId);
        _setTokenURI(agentId, newURI);
        emit URIUpdated(agentId, newURI, msg.sender);
    }

    function getMetadata(uint256 agentId, string memory metadataKey) external view returns (bytes memory) {
        _requireOwned(agentId);
        if (keccak256(bytes(metadataKey)) == AGENT_WALLET_KEY) {
            return abi.encode(_agentWallets[agentId]);
        }
        return _metadata[agentId][keccak256(bytes(metadataKey))];
    }

    function setMetadata(uint256 agentId, string calldata metadataKey, bytes calldata metadataValue) external {
        _requireAgentAuthorization(agentId);
        if (keccak256(bytes(metadataKey)) == AGENT_WALLET_KEY) {
            revert AgentIdentityRegistry__ReservedMetadataKey();
        }
        _setMetadata(agentId, metadataKey, metadataValue);
    }

    function getAgentWallet(uint256 agentId) external view returns (address) {
        _requireOwned(agentId);
        return _agentWallets[agentId];
    }

    function setAgentWallet(uint256 agentId, address newWallet, uint256 deadline, bytes calldata signature) external {
        _requireAgentAuthorization(agentId);
        if (newWallet == address(0)) revert AgentIdentityRegistry__ZeroAddress();
        if (block.timestamp > deadline) revert AgentIdentityRegistry__ExpiredSignature();

        bytes32 digest = agentWalletDigest(agentId, newWallet, deadline);
        bool valid = newWallet.code.length == 0
            ? ECDSA.recover(digest, signature) == newWallet
            : IERC1271(newWallet).isValidSignature(digest, signature) == ERC1271_MAGIC_VALUE;
        if (!valid) revert AgentIdentityRegistry__InvalidSignature();

        _setAgentWallet(agentId, newWallet);
    }

    function unsetAgentWallet(uint256 agentId) external {
        _requireAgentAuthorization(agentId);
        _clearAgentWallet(agentId);
    }

    function agentWalletDigest(uint256 agentId, address newWallet, uint256 deadline) public view returns (bytes32) {
        return _hashTypedDataV4(keccak256(abi.encode(SET_AGENT_WALLET_TYPEHASH, agentId, newWallet, deadline)));
    }

    function isValidAgent(address agentWallet) external view returns (bool) {
        uint256 agentId = walletToAgentId[agentWallet];
        return agentWallet != address(0) && isRegistered[agentWallet] && _ownerOf(agentId) != address(0)
            && _agentWallets[agentId] == agentWallet;
    }

    function _register(address agentOwner, string memory agentURI, MetadataEntry[] memory metadata)
        internal
        returns (uint256 agentId)
    {
        if (agentOwner == address(0)) revert AgentIdentityRegistry__ZeroAddress();
        if (isRegistered[agentOwner]) revert AgentIdentityRegistry__AlreadyRegistered(agentOwner);

        agentId = _nextTokenId++;
        _mint(agentOwner, agentId);
        if (bytes(agentURI).length != 0) _setTokenURI(agentId, agentURI);
        _setAgentWallet(agentId, agentOwner);

        for (uint256 i = 0; i < metadata.length; i++) {
            if (keccak256(bytes(metadata[i].metadataKey)) == AGENT_WALLET_KEY) {
                revert AgentIdentityRegistry__ReservedMetadataKey();
            }
            _setMetadata(agentId, metadata[i].metadataKey, metadata[i].metadataValue);
        }

        emit Registered(agentId, agentURI, agentOwner);
    }

    function _setMetadata(uint256 agentId, string memory metadataKey, bytes memory metadataValue) internal {
        _metadata[agentId][keccak256(bytes(metadataKey))] = metadataValue;
        emit MetadataSet(agentId, metadataKey, metadataKey, metadataValue);
    }

    function _setAgentWallet(uint256 agentId, address newWallet) internal {
        uint256 existingAgentId = walletToAgentId[newWallet];
        if (isRegistered[newWallet] && existingAgentId != agentId) {
            revert AgentIdentityRegistry__AlreadyRegistered(newWallet);
        }

        _clearAgentWallet(agentId);
        _agentWallets[agentId] = newWallet;
        walletToAgentId[newWallet] = agentId;
        isRegistered[newWallet] = true;
        emit MetadataSet(agentId, "agentWallet", "agentWallet", abi.encode(newWallet));
    }

    function _clearAgentWallet(uint256 agentId) internal {
        address previousWallet = _agentWallets[agentId];
        if (previousWallet != address(0)) {
            isRegistered[previousWallet] = false;
            walletToAgentId[previousWallet] = 0;
            _agentWallets[agentId] = address(0);
            emit MetadataSet(agentId, "agentWallet", "agentWallet", abi.encode(address(0)));
        }
    }

    function _requireAgentAuthorization(uint256 agentId) internal view {
        address agentOwner = ownerOf(agentId);
        if (!_isAuthorized(agentOwner, msg.sender, agentId)) {
            revert AgentIdentityRegistry__NotAuthorized();
        }
    }

    function _update(address to, uint256 tokenId, address auth) internal override returns (address) {
        address from = _ownerOf(tokenId);
        address previousOwner = super._update(to, tokenId, auth);

        if (from != address(0) && to != from) {
            _clearAgentWallet(tokenId);
        }

        return previousOwner;
    }
}
