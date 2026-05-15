// Program IDs одинаковы для localnet и devnet (ключи из target/deploy)
export const ORACLE_PROGRAM_ID = "3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8";
export const MINTER_PROGRAM_ID = "95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc";

// Сети и RPC (переключатель на фронте)
export type NetworkId = "localnet" | "devnet";

export const NETWORKS: Record<NetworkId, { rpc: string; label: string }> = {
  localnet: { rpc: "http://127.0.0.1:8899", label: "Localnet" },
  devnet: { rpc: "https://api.devnet.solana.com", label: "Devnet" },
};

export const DEFAULT_NETWORK: NetworkId = "localnet";

/** RPC по умолчанию (для обратной совместимости) */
export const RPC_URL = NETWORKS[DEFAULT_NETWORK].rpc;

// Metaplex Token Metadata (для имени/тикера/картинки в кошельке)
export const MPL_TOKEN_METADATA_PROGRAM_ID = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

/** URL транзакции в Solana Explorer (localnet = custom RPC, devnet = devnet) */
export function txExplorerUrl(signature: string, network: NetworkId, rpcUrl: string): string {
  const base = `https://explorer.solana.com/tx/${signature}`;
  return network === "devnet"
    ? `${base}?cluster=devnet`
    : `${base}?cluster=custom&customUrl=${encodeURIComponent(rpcUrl)}`;
}

export const ORACLE_SEED = new TextEncoder().encode("oracle_state");
export const MINTER_SEED = new TextEncoder().encode("minter_config");
export const METADATA_SEED = new TextEncoder().encode("metadata");

// Discriminator для инструкции mint_token (первые 8 байт sha256("global:mint_token"))
export const MINT_TOKEN_DISCRIMINATOR = new Uint8Array([172, 137, 183, 14, 207, 110, 234, 56]);
