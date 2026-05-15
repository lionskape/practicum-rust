import { useEffect, useMemo, useState } from "react";
import { ConnectionProvider, WalletProvider } from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import { PhantomWalletAdapter, SolflareWalletAdapter } from "@solana/wallet-adapter-wallets";
import TerminalMint from "./TerminalMint";
import { DEFAULT_NETWORK, NETWORKS, type NetworkId } from "../config";

export default function TerminalApp() {
  const [mounted, setMounted] = useState(false);
  const [network, setNetwork] = useState<NetworkId>(DEFAULT_NETWORK);
  const endpoint = NETWORKS[network].rpc;

  useEffect(() => setMounted(true), []);

  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolflareWalletAdapter()],
    []
  );

  if (!mounted) {
    return (
      <div className="terminal">
        <div className="terminal-header">
          <span className="terminal-title">mini-launchpad</span>
        </div>
        <pre className="terminal-body">
          <span className="term-line term-muted">загрузка...</span>
        </pre>
      </div>
    );
  }

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <TerminalMint network={network} setNetwork={setNetwork} rpcUrl={endpoint} />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
