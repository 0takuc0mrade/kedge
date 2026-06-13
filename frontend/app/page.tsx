import KedgeDashboard from "@/components/KedgeDashboard";
import { WalletProvider } from "@/components/WalletProvider";

export default function Home() {
  return (
    <WalletProvider>
      <KedgeDashboard />
    </WalletProvider>
  );
}
