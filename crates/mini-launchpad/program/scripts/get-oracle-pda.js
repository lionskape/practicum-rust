#!/usr/bin/env node
// Run from repo root: node program/scripts/get-oracle-pda.js
// Or from program/: node scripts/get-oracle-pda.js
import anchor from "@coral-xyz/anchor";
const { PublicKey } = anchor.web3;
const ORACLE_PROGRAM_ID = new PublicKey(
  "3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8"
);
const [oraclePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("oracle_state")],
  ORACLE_PROGRAM_ID
);
console.log("ORACLE_STATE_PUBKEY=" + oraclePda.toBase58());
