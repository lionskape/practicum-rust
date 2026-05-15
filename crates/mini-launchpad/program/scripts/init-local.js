#!/usr/bin/env node
/**
 * Initialize oracle + minter on localnet. Run from program/: node scripts/init-local.js
 * Requires: validator running, programs deployed, wallet funded.
 */
import { BorshInstructionCoder } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { createRequire } from "module";
import path from "path";
import fs from "fs";
import BN from "bn.js";

const require = createRequire(import.meta.url);

const ORACLE_PROGRAM_ID = new PublicKey(
  "3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8"
);
const MINTER_PROGRAM_ID = new PublicKey(
  "95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc"
);
const ORACLE_SEED = Buffer.from("oracle_state");
const MINTER_SEED = Buffer.from("minter_config");
const INITIAL_PRICE = new BN(120_000_000);
const MINT_FEE_USD = new BN(5_000_000);

const programDir = path.resolve(process.cwd());
const walletPath =
  process.env.ANCHOR_WALLET ||
  path.join(process.env.HOME || "", ".config/solana/id.json");

async function main() {
  const rpcUrl =
    process.env.RPC_URL ||
    process.env.SOLANA_RPC_HTTP ||
    "http://127.0.0.1:8899";
  const connection = new Connection(rpcUrl);
  const payer = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf8")))
  );

  const oracleIdl = require(path.join(
    programDir,
    "target/idl/sol_usd_oracle.json"
  ));
  const minterIdl = require(path.join(
    programDir,
    "target/idl/token_minter.json"
  ));
  const oracleCoder = new BorshInstructionCoder(oracleIdl);
  const minterCoder = new BorshInstructionCoder(minterIdl);

  const [oraclePda] = PublicKey.findProgramAddressSync(
    [ORACLE_SEED],
    ORACLE_PROGRAM_ID
  );
  const [minterPda] = PublicKey.findProgramAddressSync(
    [MINTER_SEED],
    MINTER_PROGRAM_ID
  );

  console.log("ORACLE_STATE_PUBKEY=" + oraclePda.toBase58());

  const sendTx = async (tx) => {
    const sig = await sendAndConfirmTransaction(connection, tx, [payer], {
      commitment: "confirmed",
      skipPreflight: false,
      maxRetries: 10,
    });
    console.log("  tx:", sig);
    return sig;
  };

  /** На devnet даём время аккаунту появиться в состоянии перед следующей инструкцией */
  const waitForAccount = async (pubkey, label, maxAttempts = 15) => {
    if (rpcUrl.includes("devnet") || rpcUrl.includes("mainnet")) {
      for (let i = 0; i < maxAttempts; i++) {
        const info = await connection.getAccountInfo(pubkey, "confirmed");
        if (info && info.data && info.data.length > 0) {
          if (label)
            console.log(`  ${label} visible after ${i + 1} attempt(s)`);
          return;
        }
        await new Promise((r) => setTimeout(r, 1500));
      }
      if (label)
        console.warn(
          `  warning: ${label} not visible after ${maxAttempts} attempts, continuing anyway`
        );
    }
  };

  console.log("Initializing oracle...");
  const initOracleIx = {
    programId: ORACLE_PROGRAM_ID,
    keys: [
      { pubkey: oraclePda, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(
      oracleCoder.encode("initialize_oracle", { admin: payer.publicKey })
    ),
  };
  let tx = new Transaction().add(initOracleIx);
  try {
    await sendTx(tx);
  } catch (e) {
    if (
      !String(e.message || e).includes("already in use") &&
      !String(e.message || e).includes("0x0")
    )
      throw e;
    console.log("  (oracle already initialized, skipping)");
  }

  await waitForAccount(oraclePda, "Oracle account");

  console.log("Setting initial price...");
  const updateIx = {
    programId: ORACLE_PROGRAM_ID,
    keys: [
      { pubkey: oraclePda, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
    ],
    data: Buffer.from(
      oracleCoder.encode("update_price", { new_price: INITIAL_PRICE })
    ),
  };
  tx = new Transaction().add(updateIx);
  await sendTx(tx);

  console.log("Initializing minter (treasury = wallet)...");
  const initMinterIx = {
    programId: MINTER_PROGRAM_ID,
    keys: [
      { pubkey: minterPda, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(
      minterCoder.encode("initialize_minter", {
        treasury: payer.publicKey,
        mint_fee_usd: MINT_FEE_USD,
        oracle_state: oraclePda,
        oracle_program: ORACLE_PROGRAM_ID,
      })
    ),
  };
  tx = new Transaction().add(initMinterIx);
  try {
    await sendTx(tx);
  } catch (e) {
    if (
      !String(e.message || e).includes("already in use") &&
      !String(e.message || e).includes("0x0")
    )
      throw e;
    console.log("  (minter already initialized, skipping)");
  }

  console.log("Done. Add to backend/.env:");
  console.log("ORACLE_STATE_PUBKEY=" + oraclePda.toBase58());
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
