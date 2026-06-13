"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Check, LoaderCircle, Wallet, WifiOff } from "lucide-react";

const MANTLE_SEPOLIA_CHAIN_ID = "0x138b";
const CONNECT_TIMEOUT_MS = 20_000;
const REQUEST_TIMEOUT_MS = 12_000;

type EthereumProvider = {
  isBraveWallet?: boolean;
  isMetaMask?: boolean;
  isRabby?: boolean;
  providers?: EthereumProvider[];
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

type Eip6963ProviderDetail = {
  info: {
    name: string;
    rdns: string;
    uuid: string;
  };
  provider: EthereumProvider;
};

declare global {
  interface Window {
    ethereum?: EthereumProvider;
  }
}

function shortAddress(address: string) {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

function providerRank(provider: EthereumProvider) {
  if (provider.isMetaMask && !provider.isBraveWallet) return 0;
  if (provider.isRabby) return 1;
  if (provider.isBraveWallet) return 2;
  return 3;
}

function selectProvider(
  announcedProviders: EthereumProvider[] = [],
): EthereumProvider | undefined {
  const injected = window.ethereum;
  const providers = [
    ...announcedProviders,
    ...(injected?.providers ?? []),
    ...(injected ? [injected] : []),
  ];
  const unique = providers.filter(
    (provider, index) => providers.indexOf(provider) === index,
  );

  return unique.sort((a, b) => providerRank(a) - providerRank(b))[0];
}

function requestWithTimeout<T>(
  provider: EthereumProvider,
  method: string,
  params?: unknown[] | Record<string, unknown>,
  timeoutMs = REQUEST_TIMEOUT_MS,
) {
  return Promise.race([
    provider.request({ method, params }) as Promise<T>,
    new Promise<never>((_, reject) => {
      window.setTimeout(
        () => reject(new Error(`Wallet request timed out: ${method}`)),
        timeoutMs,
      );
    }),
  ]);
}

function walletErrorMessage(error: unknown) {
  const code =
    typeof error === "object" && error !== null && "code" in error
      ? Number(error.code)
      : 0;
  const message =
    error instanceof Error
      ? error.message.toLowerCase()
      : typeof error === "object" && error !== null && "message" in error
        ? String(error.message).toLowerCase()
        : "";

  if (code === 4001) return "Connection declined";
  if (code === -32002 || message.includes("already pending")) {
    return "Open wallet to finish";
  }
  if (message.includes("timed out")) return "Open wallet and retry";
  if (message.includes("unexpected error")) return "Wallet extension failed";
  return "Wallet unavailable";
}

async function ensureMantleSepolia(provider: EthereumProvider) {
  const chainId = await requestWithTimeout<string>(provider, "eth_chainId");

  if (chainId.toLowerCase() === MANTLE_SEPOLIA_CHAIN_ID) return;

  try {
    await requestWithTimeout(provider, "wallet_switchEthereumChain", [
      { chainId: MANTLE_SEPOLIA_CHAIN_ID },
    ]);
  } catch (error) {
    const code =
      typeof error === "object" && error !== null && "code" in error
        ? Number(error.code)
        : 0;

    if (code !== 4902) throw error;

    await requestWithTimeout(provider, "wallet_addEthereumChain", [
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
    ]);
  }
}

export default function WalletButton() {
  const providerRef = useRef<EthereumProvider | undefined>(undefined);
  const [address, setAddress] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [message, setMessage] = useState("");

  const syncAccount = useCallback(async () => {
    const provider = providerRef.current ?? selectProvider();
    if (!provider) return;
    providerRef.current = provider;

    try {
      const accounts = await requestWithTimeout<string[]>(
        provider,
        "eth_accounts",
      );
      setAddress(accounts[0] ?? "");
    } catch {
      setAddress("");
    }
  }, []);

  useEffect(() => {
    const announcedProviders: EthereumProvider[] = [];
    let activeProvider: EthereumProvider | undefined;

    const bindProvider = (provider: EthereumProvider) => {
      if (activeProvider === provider) return;
      if (activeProvider) {
        activeProvider.removeListener?.("accountsChanged", handleAccounts);
        activeProvider.removeListener?.("chainChanged", handleChain);
      }
      activeProvider = provider;
      providerRef.current = provider;
      provider.on?.("accountsChanged", handleAccounts);
      provider.on?.("chainChanged", handleChain);
      void syncAccount();
    };

    const handleAnnouncement = (event: Event) => {
      const detail = (event as CustomEvent<Eip6963ProviderDetail>).detail;
      if (!detail?.provider || announcedProviders.includes(detail.provider)) {
        return;
      }
      announcedProviders.push(detail.provider);
      const selected = selectProvider(announcedProviders);
      if (selected) bindProvider(selected);
    };

    const handleAccounts = (...args: unknown[]) => {
      const accounts = args[0] as string[];
      setAddress(accounts[0] ?? "");
    };
    const handleChain = () => {
      setMessage("");
      void syncAccount();
    };

    window.addEventListener(
      "eip6963:announceProvider",
      handleAnnouncement as EventListener,
    );
    window.dispatchEvent(new Event("eip6963:requestProvider"));

    const selected = selectProvider(announcedProviders);
    if (selected) bindProvider(selected);

    return () => {
      window.removeEventListener(
        "eip6963:announceProvider",
        handleAnnouncement as EventListener,
      );
      activeProvider?.removeListener?.("accountsChanged", handleAccounts);
      activeProvider?.removeListener?.("chainChanged", handleChain);
    };
  }, [syncAccount]);

  const connect = async () => {
    const provider = providerRef.current ?? selectProvider();
    if (!provider) {
      setMessage("Install MetaMask");
      window.open("https://metamask.io/download/", "_blank", "noreferrer");
      return;
    }
    providerRef.current = provider;

    setConnecting(true);
    setMessage("");
    try {
      const accounts = await requestWithTimeout<string[]>(
        provider,
        "eth_requestAccounts",
        undefined,
        CONNECT_TIMEOUT_MS,
      );
      await ensureMantleSepolia(provider);
      setAddress(accounts[0] ?? "");
    } catch (error) {
      setMessage(walletErrorMessage(error));
    } finally {
      setConnecting(false);
    }
  };

  return (
    <button
      className={address ? "wallet-button wallet-connected" : "wallet-button"}
      type="button"
      onClick={connect}
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
