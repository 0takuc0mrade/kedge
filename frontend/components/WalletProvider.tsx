"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

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
  provider: EthereumProvider;
};

type WalletContextValue = {
  address: string;
  connecting: boolean;
  message: string;
  connect: () => Promise<string>;
  signMessage: (message: string) => Promise<string>;
};

declare global {
  interface Window {
    ethereum?: EthereumProvider;
  }
}

const WalletContext = createContext<WalletContextValue | null>(null);

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

function utf8ToHex(value: string) {
  return `0x${Array.from(new TextEncoder().encode(value))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("")}`;
}

export function WalletProvider({ children }: { children: React.ReactNode }) {
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

    const handleAccounts = (...args: unknown[]) => {
      const accounts = args[0] as string[];
      setAddress(accounts[0] ?? "");
    };
    const handleChain = () => {
      setMessage("");
      void syncAccount();
    };
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

  const connect = useCallback(async () => {
    const provider = providerRef.current ?? selectProvider();
    if (!provider) {
      setMessage("Install MetaMask");
      window.open("https://metamask.io/download/", "_blank", "noreferrer");
      throw new Error("No injected wallet found");
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
      const account = accounts[0] ?? "";
      setAddress(account);
      return account;
    } catch (error) {
      setMessage(walletErrorMessage(error));
      throw error;
    } finally {
      setConnecting(false);
    }
  }, []);

  const signMessage = useCallback(
    async (value: string) => {
      const provider = providerRef.current ?? selectProvider();
      if (!provider) throw new Error("No injected wallet found");
      const account = address || (await connect());
      await ensureMantleSepolia(provider);
      return requestWithTimeout<string>(
        provider,
        "personal_sign",
        [utf8ToHex(value), account],
        CONNECT_TIMEOUT_MS,
      );
    },
    [address, connect],
  );

  const contextValue = useMemo(
    () => ({ address, connecting, message, connect, signMessage }),
    [address, connect, connecting, message, signMessage],
  );

  return (
    <WalletContext.Provider value={contextValue}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error("useWallet must be used inside WalletProvider");
  }
  return context;
}
