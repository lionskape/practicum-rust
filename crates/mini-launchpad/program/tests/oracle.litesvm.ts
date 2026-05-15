import * as anchor from "@coral-xyz/anchor";
import {
  BorshAccountsCoder,
  BorshInstructionCoder,
  type Idl,
} from "@coral-xyz/anchor";
import { LiteSVM } from "litesvm";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { expect } from "chai";
import path from "path";
import { createRequire } from "module";
import { fileURLToPath } from "url";
import BN from "bn.js";
const require = createRequire(import.meta.url);
const oracleIdl = require("../target/idl/sol_usd_oracle.json");

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ORACLE_PROGRAM_ID = new PublicKey(
  "3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8"
);
const ORACLE_SO = path.resolve(__dirname, "../target/deploy/sol_usd_oracle.so");
const ORACLE_SEED = Buffer.from("oracle_state");

const coder = new BorshInstructionCoder(oracleIdl as Idl);
const accountsCoder = new BorshAccountsCoder(oracleIdl as Idl);

function assertSuccess(res: any) {
  if (typeof res?.err === "function") {
    const err = res.err();
    expect(err, `tx failed: ${res.toString()}`).to.be.null;
  }
}

function assertFailure(res: any) {
  if (typeof res?.err === "function") {
    const err = res.err();
    expect(err, `tx unexpectedly succeeded: ${res.toString()}`).to.not.be.null;
    return;
  }
  throw new Error("LiteSVM result does not expose err()");
}

function buildTx(
  ixs: TransactionInstruction[],
  feePayer: Keypair,
  svm: LiteSVM
) {
  const tx = new Transaction({
    feePayer: feePayer.publicKey,
    recentBlockhash: svm.latestBlockhash(),
  });
  tx.add(...ixs);
  tx.sign(feePayer);
  return tx;
}

describe("sol_usd_oracle (LiteSVM)", () => {
  const svm = new LiteSVM()
    .withSysvars()
    .withBuiltins()
    .withDefaultPrograms()
    .withBlockhashCheck(false)
    .withSigverify(false);
  svm.addProgramFromFile(ORACLE_PROGRAM_ID, ORACLE_SO);

  const payer = Keypair.generate();
  const attacker = Keypair.generate();
  const [oraclePda] = PublicKey.findProgramAddressSync(
    [ORACLE_SEED],
    ORACLE_PROGRAM_ID
  );

  before(() => {
    svm.airdrop(payer.publicKey, BigInt(5_000_000_000));
    svm.airdrop(attacker.publicKey, BigInt(1_000_000_000));
  });

  it("initialize_oracle sets admin and defaults", () => {
    const data = coder.encode("initialize_oracle", { admin: payer.publicKey });
    const ix = new TransactionInstruction({
      programId: ORACLE_PROGRAM_ID,
      keys: [
        { pubkey: oraclePda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    });

    const res = svm.sendTransaction(buildTx([ix], payer, svm));
    assertSuccess(res);

    const acct = svm.getAccount(oraclePda);
    expect(acct).to.not.be.null;
    const decoded = accountsCoder.decode(
      "OracleState",
      Buffer.from((acct as any).data)
    );
    expect(decoded.admin.toBase58()).to.eq(payer.publicKey.toBase58());
    expect(decoded.price.toNumber()).to.eq(0);
    expect(decoded.decimals).to.eq(6);
  });

  it("update_price updates price only for admin", () => {
    const newPrice = new BN(123_000_000);
    const data = coder.encode("update_price", { new_price: newPrice });
    const ix = new TransactionInstruction({
      programId: ORACLE_PROGRAM_ID,
      keys: [
        { pubkey: oraclePda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    });

    const res = svm.sendTransaction(buildTx([ix], payer, svm));
    assertSuccess(res);

    const acct = svm.getAccount(oraclePda);
    const decoded = accountsCoder.decode(
      "OracleState",
      Buffer.from((acct as any).data)
    );
    expect(decoded.price.toString()).to.eq(newPrice.toString());
  });

  it("rejects update_price from non-admin signer", () => {
    const data = coder.encode("update_price", {
      new_price: new BN(999_000_000),
    });
    const ix = new TransactionInstruction({
      programId: ORACLE_PROGRAM_ID,
      keys: [
        { pubkey: oraclePda, isSigner: false, isWritable: true },
        { pubkey: attacker.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    });

    const res = svm.sendTransaction(buildTx([ix], attacker, svm));
    assertFailure(res);

    const acct = svm.getAccount(oraclePda);
    const decoded = accountsCoder.decode(
      "OracleState",
      Buffer.from((acct as any).data)
    );
    expect(decoded.price.toString()).to.eq("123000000");
  });

  it("rejects zero price update", () => {
    const data = coder.encode("update_price", { new_price: new BN(0) });
    const ix = new TransactionInstruction({
      programId: ORACLE_PROGRAM_ID,
      keys: [
        { pubkey: oraclePda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    });

    const res = svm.sendTransaction(buildTx([ix], payer, svm));
    assertFailure(res);

    const acct = svm.getAccount(oraclePda);
    const decoded = accountsCoder.decode(
      "OracleState",
      Buffer.from((acct as any).data)
    );
    expect(decoded.price.toString()).to.eq("123000000");
  });
});
