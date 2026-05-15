import {
  getAssociatedTokenAddressSync,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  MINTER_PROGRAM_ID,
  MPL_TOKEN_METADATA_PROGRAM_ID,
  ORACLE_PROGRAM_ID,
  ORACLE_SEED,
  MINTER_SEED,
  METADATA_SEED,
  MINT_TOKEN_DISCRIMINATOR,
} from "./config";

const ORACLE_PK = new PublicKey(ORACLE_PROGRAM_ID);
const MINTER_PK = new PublicKey(MINTER_PROGRAM_ID);
const MPL_METADATA_PK = new PublicKey(MPL_TOKEN_METADATA_PROGRAM_ID);
const SYSVAR_RENT = new PublicKey("SysvarRent111111111111111111111111111111111");

/** Borsh: u32 LE length + UTF-8 bytes */
function encodeBorshString(s: string): Uint8Array {
  const utf8 = new TextEncoder().encode(s);
  const out = new Uint8Array(4 + utf8.length);
  new DataView(out.buffer, out.byteOffset, out.byteLength).setUint32(0, utf8.length, true);
  out.set(utf8, 4);
  return out;
}

type BuildMintTokenInstructionArgs = {
  user: PublicKey;
  mintKeypair: Keypair;
  treasury: PublicKey;
  decimals: number;
  initialSupply: bigint;
  name: string;
  symbol: string;
  uri: string;
};

export function buildMintTokenInstruction({
  user,
  mintKeypair,
  treasury,
  decimals,
  initialSupply,
  name,
  symbol,
  uri,
}: BuildMintTokenInstructionArgs): TransactionInstruction {
  const [configPda] = PublicKey.findProgramAddressSync([MINTER_SEED], MINTER_PK);
  const [oraclePda] = PublicKey.findProgramAddressSync([ORACLE_SEED], ORACLE_PK);
  const [metadataPda] = PublicKey.findProgramAddressSync(
    [METADATA_SEED, MPL_METADATA_PK.toBytes(), mintKeypair.publicKey.toBytes()],
    MPL_METADATA_PK
  );
  const userAta = getAssociatedTokenAddressSync(mintKeypair.publicKey, user);

  const nameEnc = encodeBorshString(name.slice(0, 32));
  const symbolEnc = encodeBorshString(symbol.slice(0, 10));
  const uriEnc = encodeBorshString(uri.slice(0, 200));
  const data = new Uint8Array(17 + nameEnc.length + symbolEnc.length + uriEnc.length);
  data.set(MINT_TOKEN_DISCRIMINATOR, 0);
  data[8] = decimals;
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  view.setBigUint64(9, initialSupply, true);
  let off = 17;
  data.set(nameEnc, off);
  off += nameEnc.length;
  data.set(symbolEnc, off);
  off += symbolEnc.length;
  data.set(uriEnc, off);

  return new TransactionInstruction({
    programId: MINTER_PK,
    keys: [
      { pubkey: configPda, isSigner: false, isWritable: true },
      { pubkey: user, isSigner: true, isWritable: true },
      { pubkey: treasury, isSigner: false, isWritable: true },
      { pubkey: ORACLE_PK, isSigner: false, isWritable: false },
      { pubkey: oraclePda, isSigner: false, isWritable: false },
      { pubkey: mintKeypair.publicKey, isSigner: true, isWritable: true },
      { pubkey: userAta, isSigner: false, isWritable: true },
      { pubkey: MPL_METADATA_PK, isSigner: false, isWritable: false },
      { pubkey: metadataPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT, isSigner: false, isWritable: false },
    ],
    data: data as Buffer,
  });
}
