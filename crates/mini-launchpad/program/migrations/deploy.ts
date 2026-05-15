import * as anchor from "@coral-xyz/anchor";
import path from "path";
import { fileURLToPath } from "url";
import { createRequire } from "module";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const { PublicKey } = anchor.web3;

const ORACLE_PROGRAM_ID = new PublicKey(
  "3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8"
);
const MINTER_PROGRAM_ID = new PublicKey(
  "95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc"
);
const ORACLE_SEED = Buffer.from("oracle_state");
const MINTER_SEED = Buffer.from("minter_config");
const INITIAL_PRICE = new anchor.BN(120_000_000); // $120 * 1e6
const MINT_FEE_USD = new anchor.BN(5_000_000); // $5 * 1e6

module.exports = async function (provider: anchor.AnchorProvider) {
  anchor.setProvider(provider);
  const wallet = (provider.wallet as anchor.Wallet).publicKey;

  const oracleIdl = require(path.join(
    process.cwd(),
    "target/idl/sol_usd_oracle.json"
  ));
  const minterIdl = require(path.join(
    process.cwd(),
    "target/idl/token_minter.json"
  ));

  const oracleProgram = new anchor.Program(
    oracleIdl,
    ORACLE_PROGRAM_ID,
    provider
  );
  const minterProgram = new anchor.Program(
    minterIdl,
    MINTER_PROGRAM_ID,
    provider
  );

  const [oraclePda] = PublicKey.findProgramAddressSync(
    [ORACLE_SEED],
    ORACLE_PROGRAM_ID
  );
  const [minterPda] = PublicKey.findProgramAddressSync(
    [MINTER_SEED],
    MINTER_PROGRAM_ID
  );

  console.log("ORACLE_STATE_PUBKEY=" + oraclePda.toBase58());
  console.log("Initializing oracle...");
  await oracleProgram.methods
    .initializeOracle(wallet)
    .accounts({
      oracle: oraclePda,
      payer: wallet,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Setting initial price...");
  await oracleProgram.methods
    .updatePrice(INITIAL_PRICE)
    .accounts({ oracle: oraclePda, admin: wallet })
    .rpc();

  console.log("Initializing minter (treasury = wallet)...");
  await minterProgram.methods
    .initializeMinter(wallet, MINT_FEE_USD, oraclePda, ORACLE_PROGRAM_ID)
    .accounts({
      config: minterPda,
      admin: wallet,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Done. Add to backend/.env:");
  console.log("ORACLE_STATE_PUBKEY=" + oraclePda.toBase58());
};
