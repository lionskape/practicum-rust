import { useCallback, useEffect, useState } from "react";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import {
  ORACLE_PROGRAM_ID,
  MINTER_PROGRAM_ID,
  ORACLE_SEED,
  MINTER_SEED,
  NETWORKS,
  txExplorerUrl,
  type NetworkId,
} from "../config";
import { buildMintTokenInstruction } from "../mintInstruction";

const ORACLE_PK = new PublicKey(ORACLE_PROGRAM_ID);
const MINTER_PK = new PublicKey(MINTER_PROGRAM_ID);

function useOracleAndMinter(rpcUrl: string) {
  const [oraclePrice, setOraclePrice] = useState<number | null>(null);
  const [feeUsd, setFeeUsd] = useState<number | null>(null);
  const [treasury, setTreasury] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      setError(null);
      const conn = new Connection(rpcUrl);
      const [oraclePda] = PublicKey.findProgramAddressSync([ORACLE_SEED], ORACLE_PK);
      const [minterPda] = PublicKey.findProgramAddressSync([MINTER_SEED], MINTER_PK);
      const [oracleAcc, minterAcc] = await Promise.all([
        conn.getAccountInfo(oraclePda),
        conn.getAccountInfo(minterPda),
      ]);
      if (oracleAcc?.data && oracleAcc.data.length >= 48) {
        const data = oracleAcc.data as Uint8Array;
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const price = view.getBigUint64(40, true);
        setOraclePrice(Number(price));
      } else setOraclePrice(null);
      if (minterAcc?.data && minterAcc.data.length >= 80) {
        const data = minterAcc.data as Uint8Array;
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        const fee = view.getBigUint64(72, true);
        setFeeUsd(Number(fee));
        const treasuryPubkey = new PublicKey(data.subarray(40, 72));
        setTreasury(treasuryPubkey.toBase58());
      } else {
        setFeeUsd(null);
        setTreasury(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [rpcUrl]);

  useEffect(() => {
    fetchData();
    const t = setInterval(fetchData, 5000);
    return () => clearInterval(t);
  }, [fetchData]);

  return { oraclePrice, feeUsd, treasury, loading, error };
}

type TerminalMintProps = {
  network: NetworkId;
  setNetwork: (n: NetworkId) => void;
  rpcUrl: string;
};

type MintedToken = {
  mint: string;
  name: string;
  symbol: string;
  imageUrl: string | null;
  txSig: string;
};

export default function TerminalMint({ network, setNetwork, rpcUrl }: TerminalMintProps) {
  const { publicKey, connected, sendTransaction, disconnect, connecting } = useWallet();
  const { oraclePrice, feeUsd, treasury, loading, error } = useOracleAndMinter(rpcUrl);
  const [decimals, setDecimals] = useState(6);
  const [supply, setSupply] = useState("1000000");
  const [tokenName, setTokenName] = useState("");
  const [symbol, setSymbol] = useState("");
  const [imageUrl, setImageUrl] = useState("");
  const [metadataUri, setMetadataUri] = useState("");
  const [useMetaplex, setUseMetaplex] = useState(false);
  const [txStatus, setTxStatus] = useState<string | null>(null);
  const [minting, setMinting] = useState(false);
  const [lastMinted, setLastMinted] = useState<MintedToken | null>(null);

  const handleMint = useCallback(async () => {
    if (!publicKey || !treasury || oraclePrice == null || oraclePrice === 0) {
      setTxStatus("Оракул или казна не готовы.");
      return;
    }
    const supplyNum = BigInt(supply);
    if (supplyNum <= 0n) {
      setTxStatus("Эмиссия должна быть > 0");
      return;
    }
    setMinting(true);
    setTxStatus(null);
    try {
      const conn = new Connection(rpcUrl);
      const mintKeypair = Keypair.generate();
      const treasuryPk = new PublicKey(treasury);
      const nameStr = tokenName.trim().slice(0, 32) || "";
      const symbolStr = symbol.trim().slice(0, 10) || "";
      const imageStr = imageUrl.trim() || "";
      const customUri = metadataUri.trim().slice(0, 200);
      const useMeta = useMetaplex && (nameStr || symbolStr || imageStr || customUri);
      const uri = useMeta
        ? customUri
          ? customUri
          : `data:application/json,${encodeURIComponent(
              JSON.stringify({
                name: nameStr || "Token",
                symbol: symbolStr || "TKN",
                image: imageStr,
              })
            )}`.slice(0, 200)
        : "";
      const ix = buildMintTokenInstruction({
        user: publicKey,
        mintKeypair,
        treasury: treasuryPk,
        decimals,
        initialSupply: supplyNum,
        name: useMeta ? nameStr : "",
        symbol: useMeta ? symbolStr : "",
        uri,
      });
      const tx = new Transaction().add(ix);
      const { blockhash, lastValidBlockHeight } = await conn.getLatestBlockhash("confirmed");
      tx.recentBlockhash = blockhash;
      tx.feePayer = publicKey;
      tx.partialSign(mintKeypair);

      const sig = await sendTransaction(tx, conn, {
        skipPreflight: true,
        preflightCommitment: "confirmed",
        maxRetries: 3,
      });
      await conn.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, "confirmed");
      setLastMinted({
        mint: mintKeypair.publicKey.toBase58(),
        name: tokenName.trim() || "Без названия",
        symbol: symbol.trim().toUpperCase() || "???",
        imageUrl: imageUrl.trim() || null,
        txSig: sig,
      });
      setTxStatus(`Минт OK: ${sig}`);
    } catch (e: unknown) {
      let msg = "Неизвестная ошибка";
      if (e instanceof Error) {
        msg = e.message;
        const cause = (e as Error & { cause?: unknown }).cause;
        if (cause instanceof Error) msg += ` (${cause.message})`;
      }
      setTxStatus(`Ошибка: ${msg}`);
    } finally {
      setMinting(false);
    }
  }, [publicKey, treasury, oraclePrice, decimals, supply, tokenName, symbol, imageUrl, metadataUri, useMetaplex, rpcUrl]);

  const explorerUrl = (mint: string) =>
    network === "devnet"
      ? `https://explorer.solana.com/address/${mint}?cluster=devnet`
      : `https://explorer.solana.com/address/${mint}?cluster=custom&customUrl=${encodeURIComponent(rpcUrl)}`;

  const txUrl = lastMinted && txStatus?.startsWith("Минт OK:")
    ? txExplorerUrl(lastMinted.txSig, network, rpcUrl)
    : null;

  return (
    <div className="terminal">
      <div className="terminal-header">
        <span className="terminal-title">mini-launchpad@{network}</span>
        <label className="term-network-switcher" style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span className="term-muted">Сеть:</span>
          <select
            value={network}
            onChange={(e) => setNetwork(e.target.value as NetworkId)}
            className="term-select"
            style={{ padding: "2px 6px", fontFamily: "inherit" }}
          >
            {(Object.keys(NETWORKS) as NetworkId[]).map((id) => (
              <option key={id} value={id}>
                {NETWORKS[id].label}
              </option>
            ))}
          </select>
        </label>
        <WalletMultiButton className="wallet-btn" />
        {(connected || connecting) && (
          <button
            type="button"
            onClick={() => disconnect()}
            className="term-btn term-btn-ghost"
            title="Отключить кошелёк (если зависло в connecting — нажми и подключи заново)"
          >
            {connecting ? "Сбросить" : "Отключить"}
          </button>
        )}
      </div>
      <pre className="terminal-body">
        <span className="term-line">$ подключи кошелёк, чтобы отминтить SPL-токен (комиссия в SOL)</span>
        <span className="term-line term-muted">
          {network === "localnet"
            ? "  → В кошельке укажи RPC http://127.0.0.1:8899 и пополни SOL."
            : "  → В кошельке выбери Devnet и пополни SOL (solana airdrop 2)."}
        </span>
        {!connected && (
          <span className="term-line term-muted">  → Кошелёк не подключён. Нажми кнопку выше.</span>
        )}
        {loading && <span className="term-line term-amber">  загрузка оракула и минтера...</span>}
        {error && <span className="term-line term-red">  ошибка: {error}</span>}
        {!loading && !error && (
          <>
            <span className="term-line term-green">  [контракты ок] оракул и минтер с цепи</span>
            <span className="term-line">  цена SOL из оракула (×10⁶): {(oraclePrice ?? 0) / 1e6} USD</span>
            <span className="term-line">  комиссия за минт: {(feeUsd ?? 0) / 1e6} USD</span>
            <span className="term-line">  казна: {treasury ?? "—"}</span>
          </>
        )}
        <span className="term-line">$ минт токена (токены придут на подключённый кошелёк)</span>
        {connected && publicKey && (
          <span className="term-line term-muted">  → Твой кошелёк: {publicKey.toBase58().slice(0, 12)}…{publicKey.toBase58().slice(-8)}</span>
        )}
        <span className="term-line term-muted">  название (для отображения):</span>
        <input
          type="text"
          placeholder="Мой токен"
          value={tokenName}
          onChange={(e) => setTokenName(e.target.value)}
          className="term-input"
          disabled={!connected || minting}
        />
        <span className="term-line term-muted">  тикер (symbol):</span>
        <input
          type="text"
          placeholder="МТ"
          maxLength={10}
          value={symbol}
          onChange={(e) => setSymbol(e.target.value.toUpperCase())}
          className="term-input term-input-short"
          disabled={!connected || minting}
        />
        <span className="term-line term-muted">  URL картинки (иконка в приложении):</span>
        <input
          type="url"
          placeholder="https://..."
          value={imageUrl}
          onChange={(e) => setImageUrl(e.target.value)}
          className="term-input"
          disabled={!connected || minting}
        />
        <span className="term-line term-muted">
          {"  URL JSON метаданных (HTTPS, до 200 символов). "}
          <a href="https://developers.metaplex.com/token-metadata/token-standard#the-fungible-standard" target="_blank" rel="noopener noreferrer" className="term-link">Формат JSON (Metaplex)</a>
        </span>
        <input
          type="url"
          placeholder="Оставь пустым — подставится JSON из полей выше (name, symbol, image); кошелёк может не показать"
          value={metadataUri}
          onChange={(e) => setMetadataUri(e.target.value)}
          className="term-input"
          disabled={!connected || minting}
        />
        <span className="term-line term-muted">  → Картинка из «URL картинки» уже попадает в наш JSON. Если поле выше пустое, этот JSON уходит как data URI — многие кошельки его не загружают, поэтому картинка может не отображаться. Чтобы показывалась: залей такой же JSON на HTTPS и вставь ссылку сюда.</span>
        <span className="term-line term-muted">  записать имя/тикер/картинку в сеть (Metaplex):</span>
        <label className="term-line" style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <input
            type="checkbox"
            checked={useMetaplex}
            onChange={(e) => setUseMetaplex(e.target.checked)}
            disabled={!connected || minting}
          />
          включить (нужен <code>make validator-metaplex</code>, иначе минт упадёт)
        </label>
        <span className="term-line term-muted">  знаков после запятой (0–9):</span>
        <input
          type="number"
          min={0}
          max={9}
          value={decimals}
          onChange={(e) => setDecimals(Number(e.target.value) || 0)}
          className="term-input term-input-short"
          disabled={!connected || minting}
        />
        <span className="term-line term-muted">  начальная эмиссия (в единицах):</span>
        <input
          type="text"
          value={supply}
          onChange={(e) => setSupply(e.target.value)}
          className="term-input term-input-short"
          disabled={!connected || minting}
        />
        <span className="term-line">
          <button
            onClick={handleMint}
            disabled={!connected || minting}
            className="term-btn"
          >
            {minting ? "минтим..." : "отминтить"}
          </button>
          {connected && !minting && (loading || !treasury || oraclePrice == null || oraclePrice === 0) && (
            <span className="term-line term-muted">  → Ожидание оракула и казны с цепи… Запустите валидатор и выполните make init.</span>
          )}
        </span>
        {txStatus && (
          <span className={`term-line ${txStatus.startsWith("Ошибка") ? "term-red" : "term-green"}`}>
            {txUrl ? (
              <>
                Минт OK:{" "}
                <a href={txUrl} target="_blank" rel="noopener noreferrer" className="term-link">
                  {lastMinted!.txSig.slice(0, 16)}…{lastMinted!.txSig.slice(-16)}
                </a>
              </>
            ) : (
              txStatus
            )}
          </span>
        )}
        {lastMinted && publicKey && (
          <div className="term-token-card">
            <span className="term-line term-green">  ✓ Токены отминчены на твой кошелёк (ATA)</span>
            <span className="term-line term-muted">  → Получатель (подписант): {publicKey.toBase58().slice(0, 8)}…{publicKey.toBase58().slice(-8)} — сверь с адресом в кошельке.</span>
            <span className="term-line term-muted">  → Если токен не виден: добавь токен в кошельке по адресу минта (кнопка ниже).</span>
            <div className="token-card">
              <div className="token-card-icon">
                {lastMinted.imageUrl ? (
                  <img src={lastMinted.imageUrl} alt="" className="token-card-img" />
                ) : (
                  <span className="token-card-placeholder">{lastMinted.symbol.slice(0, 2)}</span>
                )}
              </div>
              <div className="token-card-info">
                <span className="token-card-name">{lastMinted.name}</span>
                <span className="token-card-symbol">{lastMinted.symbol}</span>
                <a
                  href={explorerUrl(lastMinted.mint)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="token-card-link"
                >
                  {lastMinted.mint.slice(0, 8)}…{lastMinted.mint.slice(-8)}
                </a>
                <button
                  type="button"
                  className="term-btn term-btn-sm"
                  onClick={() => {
                    navigator.clipboard.writeText(lastMinted.mint);
                  }}
                >
                  Копировать адрес минта
                </button>
              </div>
            </div>
          </div>
        )}
      </pre>
    </div>
  );
}
