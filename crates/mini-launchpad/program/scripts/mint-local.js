#!/usr/bin/env node
/**
 * Mint a token through token_minter on localnet/devnet.
 *
 * Env:
 *   RPC_URL=http://127.0.0.1:8899
 *   ANCHOR_WALLET=~/.config/solana/id.json
 *   TOKEN_DECIMALS=6
 *   TOKEN_SUPPLY=1000000
 *   TOKEN_NAME=""
 *   TOKEN_SYMBOL=""
 *   TOKEN_URI=""
 */
import { BorshInstructionCoder } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
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
const MPL_TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);
const ORACLE_SEED = Buffer.from("oracle_state");
const MINTER_SEED = Buffer.from("minter_config");
const METADATA_SEED = Buffer.from("metadata");

const programDir = path.resolve(process.cwd());
const walletPath = (
  process.env.ANCHOR_WALLET ||
  path.join(process.env.HOME || "", ".config/solana/id.json")
).replace(/^~(?=\/)/, process.env.HOME || "");

function envString(name, fallback = "") {
  return (process.env[name] || fallback).trim();
}

async function main() {
  const rpcUrl =
    process.env.RPC_URL ||
    process.env.SOLANA_RPC_HTTP ||
    "http://127.0.0.1:8899";
  const connection = new Connection(rpcUrl, "confirmed");
  const payer = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf8")))
  );

  const minterIdl = require(path.join(
    programDir,
    "target/idl/token_minter.json"
  ));
  const minterCoder = new BorshInstructionCoder(minterIdl);

  const decimals = Number(process.env.TOKEN_DECIMALS || "6");
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 9) {
    throw new Error("TOKEN_DECIMALS must be an integer from 0 to 9");
  }
  const supply = new BN(process.env.TOKEN_SUPPLY || "1000000");
  const name = envString("TOKEN_NAME").slice(0, 32);
  const symbol = envString("TOKEN_SYMBOL").slice(0, 10);
  const uri = envString("TOKEN_URI").slice(0, 200);

  const [oraclePda] = PublicKey.findProgramAddressSync(
    [ORACLE_SEED],
    ORACLE_PROGRAM_ID
  );
  const [minterPda] = PublicKey.findProgramAddressSync(
    [MINTER_SEED],
    MINTER_PROGRAM_ID
  );
  const mint = Keypair.generate();
  const userAta = getAssociatedTokenAddressSync(
    mint.publicKey,
    payer.publicKey
  );
  const [metadataPda] = PublicKey.findProgramAddressSync(
    [
      METADATA_SEED,
      MPL_TOKEN_METADATA_PROGRAM_ID.toBytes(),
      mint.publicKey.toBytes(),
    ],
    MPL_TOKEN_METADATA_PROGRAM_ID
  );

  const ix = new TransactionInstruction({
    programId: MINTER_PROGRAM_ID,
    keys: [
      { pubkey: minterPda, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: payer.publicKey, isSigner: false, isWritable: true },
      { pubkey: ORACLE_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: oraclePda, isSigner: false, isWritable: false },
      { pubkey: mint.publicKey, isSigner: true, isWritable: true },
      { pubkey: userAta, isSigner: false, isWritable: true },
      {
        pubkey: MPL_TOKEN_METADATA_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      { pubkey: metadataPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      {
        pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ],
    data: minterCoder.encode("mint_token", {
      decimals,
      initial_supply: supply,
      name,
      symbol,
      uri,
    }),
  });

  const tx = new Transaction().add(ix);
  const signature = await sendAndConfirmTransaction(
    connection,
    tx,
    [payer, mint],
    {
      commitment: "confirmed",
      skipPreflight: false,
      maxRetries: 10,
    }
  );

  const metadata = name
    ? await connection.getAccountInfo(metadataPda, "confirmed")
    : null;
  console.log("MINT_SIGNATURE=" + signature);
  console.log("MINT_PUBKEY=" + mint.publicKey.toBase58());
  console.log("USER_ATA=" + userAta.toBase58());
  if (name) {
    console.log("METADATA_PDA=" + metadataPda.toBase58());
    console.log("METADATA_OWNER=" + (metadata?.owner.toBase58() || ""));
    console.log("METADATA_DATA_LEN=" + (metadata?.data.length || 0));
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
