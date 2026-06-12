// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title SettlementVault
/// @notice Holds MockUSDT treasury and disburses funds upon verified claims.
contract SettlementVault {
    using SafeERC20 for IERC20;

    IERC20 public immutable token;
    address public immutable claimRegistry;

    error Unauthorized();
    error InsufficientFunds();
    error ZeroAddress();

    event PayoutDisbursed(address indexed claimant, uint256 amount);

    modifier onlyRegistry() {
        if (msg.sender != claimRegistry) revert Unauthorized();
        _;
    }

    constructor(IERC20 _token, address _claimRegistry) {
        if (address(_token) == address(0) || _claimRegistry == address(0)) revert ZeroAddress();

        token = _token;
        claimRegistry = _claimRegistry;
    }

    /// @notice Disburses tokens to a claimant.
    /// @dev Can only be called by the configured ClaimRegistry.
    /// @param claimant The address receiving the payout.
    /// @param amount The amount of tokens to disburse.
    function disburse(address claimant, uint256 amount) external onlyRegistry {
        if (claimant == address(0)) revert ZeroAddress();
        if (token.balanceOf(address(this)) < amount) revert InsufficientFunds();

        token.safeTransfer(claimant, amount);
        emit PayoutDisbursed(claimant, amount);
    }
}
