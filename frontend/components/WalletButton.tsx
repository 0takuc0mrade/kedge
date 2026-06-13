"use client";

import { Check, LoaderCircle, Wallet, WifiOff } from "lucide-react";
import { useWallet } from "./WalletProvider";

function shortAddress(address: string) {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

export default function WalletButton() {
  const { address, connecting, message, connect } = useWallet();

  return (
    <button
      className={address ? "wallet-button wallet-connected" : "wallet-button"}
      type="button"
      onClick={() => void connect().catch(() => undefined)}
      disabled={connecting}
      title={message || (address ? address : "Connect an observer wallet")}
    >
      {connecting ? (
        <LoaderCircle className="wallet-spinner" size={15} />
      ) : address ? (
        <Check size={14} />
      ) : message ? (
        <WifiOff size={14} />
      ) : (
        <Wallet size={14} />
      )}
      <span>
        {connecting
          ? "Connecting"
          : address
            ? shortAddress(address)
            : message || "Connect wallet"}
      </span>
    </button>
  );
}
