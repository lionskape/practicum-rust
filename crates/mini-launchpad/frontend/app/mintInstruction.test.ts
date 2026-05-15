import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { describe, expect, it } from "vitest";
import {
  MINT_TOKEN_DISCRIMINATOR,
  MINTER_PROGRAM_ID,
  MPL_TOKEN_METADATA_PROGRAM_ID,
  MINTER_SEED,
  METADATA_SEED,
  ORACLE_PROGRAM_ID,
  ORACLE_SEED,
} from "./config";
import { buildMintTokenInstruction } from "./mintInstruction";

const ORACLE_PK = new PublicKey(ORACLE_PROGRAM_ID);
const MINTER_PK = new PublicKey(MINTER_PROGRAM_ID);
const MPL_METADATA_PK = new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID);
const SYSVAR_RENT = new PublicKey("SysvarRent111111111111111111111111111111111");

function readBorshString(data: Uint8Array, offset: number): { value: string; next: number } {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const len = view.getUint32(offset, true);
  const start = offset + 4;
  const end = start + len;
  return { value: new TextDecoder().decode(data.subarray(start, end)), next: end };
}

describe("buildMintTokenInstruction", () => {
  it("encodes discriminator, decimals and initial supply", () => {
    const user = Keypair.generate().publicKey;
    const treasury = Keypair.generate().publicKey;
    const mintKeypair = Keypair.generate();
    const ix = buildMintTokenInstruction({
      user,
      mintKeypair,
      treasury,
      decimals: 6,
      initialSupply: 42n,
      name: "",
      symbol: "",
      uri: "",
    });

    expect(Array.from(ix.data.subarray(0, 8))).toEqual(Array.from(MINT_TOKEN_DISCRIMINATOR));
    expect(ix.data[8]).toBe(6);

    const view = new DataView(ix.data.buffer, ix.data.byteOffset, ix.data.byteLength);
    expect(view.getBigUint64(9, true)).toBe(42n);
  });

  it("encodes maximum u64 initial supply", () => {
    const user = Keypair.generate().publicKey;
    const treasury = Keypair.generate().publicKey;
    const mintKeypair = Keypair.generate();
    const maxU64 = (1n << 64n) - 1n;
    const ix = buildMintTokenInstruction({
      user,
      mintKeypair,
      treasury,
      decimals: 9,
      initialSupply: maxU64,
      name: "",
      symbol: "",
      uri: "",
    });

    const view = new DataView(ix.data.buffer, ix.data.byteOffset, ix.data.byteLength);
    expect(view.getBigUint64(9, true)).toBe(maxU64);
  });

  it("encodes metadata strings in Borsh order and applies client limits", () => {
    const user = Keypair.generate().publicKey;
    const treasury = Keypair.generate().publicKey;
    const mintKeypair = Keypair.generate();
    const ix = buildMintTokenInstruction({
      user,
      mintKeypair,
      treasury,
      decimals: 6,
      initialSupply: 1n,
      name: "N".repeat(40),
      symbol: "S".repeat(12),
      uri: "https://example.com/".padEnd(260, "u"),
    });

    const name = readBorshString(ix.data, 17);
    const symbol = readBorshString(ix.data, name.next);
    const uri = readBorshString(ix.data, symbol.next);

    expect(name.value).toBe("N".repeat(32));
    expect(symbol.value).toBe("S".repeat(10));
    expect(uri.value).toBe("https://example.com/".padEnd(200, "u"));
    expect(uri.next).toBe(ix.data.length);
  });

  it("uses the expected account order", () => {
    const user = Keypair.generate().publicKey;
    const treasury = Keypair.generate().publicKey;
    const mintKeypair = Keypair.generate();

    const [configPda] = PublicKey.findProgramAddressSync([MINTER_SEED], MINTER_PK);
    const [oraclePda] = PublicKey.findProgramAddressSync([ORACLE_SEED], ORACLE_PK);
    const [metadataPda] = PublicKey.findProgramAddressSync(
      [METADATA_SEED, MPL_METADATA_PK.toBytes(), mintKeypair.publicKey.toBytes()],
      MPL_METADATA_PK
    );
    const userAta = getAssociatedTokenAddressSync(mintKeypair.publicKey, user);

    const ix = buildMintTokenInstruction({
      user,
      mintKeypair,
      treasury,
      decimals: 9,
      initialSupply: 1_000_000n,
      name: "",
      symbol: "",
      uri: "",
    });

    expect(ix.programId.toBase58()).toBe(MINTER_PROGRAM_ID);
    expect(ix.keys.map((k) => k.pubkey.toBase58())).toEqual([
      configPda.toBase58(),
      user.toBase58(),
      treasury.toBase58(),
      ORACLE_PK.toBase58(),
      oraclePda.toBase58(),
      mintKeypair.publicKey.toBase58(),
      userAta.toBase58(),
      MPL_METADATA_PK.toBase58(),
      metadataPda.toBase58(),
      TOKEN_PROGRAM_ID.toBase58(),
      ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),
      SystemProgram.programId.toBase58(),
      SYSVAR_RENT.toBase58(),
    ]);
  });

  it("assigns signer and writable flags correctly", () => {
    const user = Keypair.generate().publicKey;
    const treasury = Keypair.generate().publicKey;
    const mintKeypair = Keypair.generate();
    const ix = buildMintTokenInstruction({
      user,
      mintKeypair,
      treasury,
      decimals: 3,
      initialSupply: 10n,
      name: "Test",
      symbol: "TST",
      uri: "https://example.com",
    });

    const [config, userMeta, treasuryMeta, , , mintMeta, ataMeta] = ix.keys;
    expect(config.isSigner).toBe(false);
    expect(config.isWritable).toBe(true);
    expect(userMeta.isSigner).toBe(true);
    expect(userMeta.isWritable).toBe(true);
    expect(treasuryMeta.isSigner).toBe(false);
    expect(treasuryMeta.isWritable).toBe(true);
    expect(mintMeta.isSigner).toBe(true);
    expect(mintMeta.isWritable).toBe(true);
    expect(ataMeta.isSigner).toBe(false);
    expect(ataMeta.isWritable).toBe(true);
  });
});
