"use client";

import { useCallback, useEffect, useState } from "react";
import { Check, LoaderCircle, Wallet, WifiOff } from "lucide-react";

const MANTLE_SEPOLIA_CHAIN_ID = "0x138b";

type EthereumProvider = {
  request: (args: {
    method: string;
    params?: unknown[] | Record<string, unknown>;
  }) => Promise<unknown>;
  on?: (event: string, listener: (...args: unknown[]) => void) => void;
  removeListener?: (
    event: string,
    listener: (...args: unknown[]) => void,
  ) => void;
};

declare global {
  interface Window {
    ethereum?: EthereumProvider;
  }
}

function shortAddress(address: string) {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

async function ensureMantleSepolia(provider: EthereumProvider) {
  const chainId = (await provider.request({
    method: "eth_chainId",
  })) as string;

  if (chainId.toLowerCase() === MANTLE_SEPOLIA_CHAIN_ID) return;

  try {
    await provider.request({
      method: "wallet_switchEthereumChain",
      params: [{ chainId: MANTLE_SEPOLIA_CHAIN_ID }],
    });
  } catch (error) {
    const code =
      typeof error === "object" && error !== null && "code" in error
        ? Number(error.code)
        : 0;

    if (code !== 4902) throw error;

    await provider.request({
      method: "wallet_addEthereumChain",
      params: [
        {
          chainId: MANTLE_SEPOLIA_CHAIN_ID,
          chainName: "Mantle Sepolia Testnet",
          nativeCurrency: {
            name: "Mantle",
            symbol: "MNT",
            decimals: 18,
          },
          rpcUrls: ["https://rpc.sepolia.mantle.xyz"],
          blockExplorerUrls: ["https://explorer.sepolia.mantle.xyz"],
        },
      ],
    });
  }
}

export default function WalletButton() {
  const [address, setAddress] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [message, setMessage] = useState("");

  const syncAccount = useCallback(async () => {
    if (!window.ethereum) return;

    const accounts = (await window.ethereum.request({
      method: "eth_accounts",
    })) as string[];
    setAddress(accounts[0] ?? "");
  }, []);

  useEffect(() => {
    const provider = window.ethereum;
    if (!provider) return;

    void syncAccount();

    const handleAccounts = (...args: unknown[]) => {
      const accounts = args[0] as string[];
      setAddress(accounts[0] ?? "");
    };
    const handleChain = () => {
      setMessage("");
      void syncAccount();
    };

    provider.on?.("accountsChanged", handleAccounts);
    provider.on?.("chainChanged", handleChain);

    return () => {
      provider.removeListener?.("accountsChanged", handleAccounts);
      provider.removeListener?.("chainChanged", handleChain);
    };
  }, [syncAccount]);

  const connect = async () => {
    const provider = window.ethereum;
    if (!provider) {
      setMessage("Install MetaMask");
      window.open("https://metamask.io/download/", "_blank", "noreferrer");
      return;
    }

    setConnecting(true);
    setMessage("");
    try {
      const accounts = (await provider.request({
        method: "eth_requestAccounts",
      })) as string[];
      await ensureMantleSepolia(provider);
      setAddress(accounts[0] ?? "");
    } catch {
      setMessage("Connection declined");
    } finally {
      setConnecting(false);
    }
  };

  return (
    <button
      className={address ? "wallet-button wallet-connected" : "wallet-button"}
      type="button"
      onClick={connect}
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
