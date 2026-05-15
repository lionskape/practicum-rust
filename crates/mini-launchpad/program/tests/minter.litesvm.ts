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
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";
import path from "path";
import { createRequire } from "module";
import { fileURLToPath } from "url";
import BN from "bn.js";
const require = createRequire(import.meta.url);
const oracleIdl = require("../target/idl/sol_usd_oracle.json");
const minterIdl = require("../target/idl/token_minter.json");

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ORACLE_PROGRAM_ID = new PublicKey(
  "3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8"
);
const MINTER_PROGRAM_ID = new PublicKey(
  "95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc"
);

const ORACLE_SO = path.resolve(__dirname, "../target/deploy/sol_usd_oracle.so");
const MINTER_SO = path.resolve(__dirname, "../target/deploy/token_minter.so");

const ORACLE_SEED = Buffer.from("oracle_state");
const MINTER_SEED = Buffer.from("minter_config");
const METADATA_SEED = Buffer.from("metadata");
const PRICE = new BN(120_000_000); // $120 * 1e6
const FEE_USD = new BN(5_000_000); // $5 * 1e6
const U64_MAX = new BN("18446744073709551615");
const MPL_TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

const oracleCoder = new BorshInstructionCoder(oracleIdl as Idl);
const oracleAccounts = new BorshAccountsCoder(oracleIdl as Idl);
const minterCoder = new BorshInstructionCoder(minterIdl as Idl);
const minterAccounts = new BorshAccountsCoder(minterIdl as Idl);

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
  tx.sign(
    ...[feePayer, ...ixs.flatMap((ix) => (ix as any)._additionalSigners ?? [])]
  );
  return tx;
}

type MinterFixture = {
  svm: LiteSVM;
  payer: Keypair;
  treasury: Keypair;
  oraclePda: PublicKey;
  minterPda: PublicKey;
};

function createMinterFixture(): MinterFixture {
  const svm = new LiteSVM()
    .withSysvars()
    .withBuiltins()
    .withDefaultPrograms()
    .withBlockhashCheck(false)
    .withSigverify(false);
  svm.addProgramFromFile(ORACLE_PROGRAM_ID, ORACLE_SO);
  svm.addProgramFromFile(MINTER_PROGRAM_ID, MINTER_SO);

  const payer = Keypair.generate();
  const treasury = Keypair.generate();
  svm.airdrop(payer.publicKey, BigInt(10_000_000_000));
  svm.airdrop(treasury.publicKey, BigInt(1_000_000_000));

  const [oraclePda] = PublicKey.findProgramAddressSync(
    [ORACLE_SEED],
    ORACLE_PROGRAM_ID
  );
  const [minterPda] = PublicKey.findProgramAddressSync(
    [MINTER_SEED],
    MINTER_PROGRAM_ID
  );

  return { svm, payer, treasury, oraclePda, minterPda };
}

function sendFixtureIx(
  fixture: MinterFixture,
  ix: TransactionInstruction,
  feePayer = fixture.payer
) {
  return fixture.svm.sendTransaction(buildTx([ix], feePayer, fixture.svm));
}

function initializeOracleAndMinter(
  fixture: MinterFixture,
  {
    price = PRICE,
    mintFeeUsd = FEE_USD,
    oracleState = fixture.oraclePda,
    oracleProgram = ORACLE_PROGRAM_ID,
    treasury = fixture.treasury.publicKey,
  }: {
    price?: BN | null;
    mintFeeUsd?: BN;
    oracleState?: PublicKey;
    oracleProgram?: PublicKey;
    treasury?: PublicKey;
  } = {}
) {
  const initOracleIx = new TransactionInstruction({
    programId: ORACLE_PROGRAM_ID,
    keys: [
      { pubkey: fixture.oraclePda, isSigner: false, isWritable: true },
      { pubkey: fixture.payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: oracleCoder.encode("initialize_oracle", {
      admin: fixture.payer.publicKey,
    }),
  });
  assertSuccess(sendFixtureIx(fixture, initOracleIx));

  if (price !== null) {
    const updateIx = new TransactionInstruction({
      programId: ORACLE_PROGRAM_ID,
      keys: [
        { pubkey: fixture.oraclePda, isSigner: false, isWritable: true },
        { pubkey: fixture.payer.publicKey, isSigner: true, isWritable: false },
      ],
      data: oracleCoder.encode("update_price", { new_price: price }),
    });
    assertSuccess(sendFixtureIx(fixture, updateIx));
  }

  const initMinterIx = new TransactionInstruction({
    programId: MINTER_PROGRAM_ID,
    keys: [
      { pubkey: fixture.minterPda, isSigner: false, isWritable: true },
      { pubkey: fixture.payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: minterCoder.encode("initialize_minter", {
      treasury,
      mint_fee_usd: mintFeeUsd,
      oracle_state: oracleState,
      oracle_program: oracleProgram,
    }),
  });
  assertSuccess(sendFixtureIx(fixture, initMinterIx));
}

function buildMintInstruction(
  fixture: MinterFixture,
  {
    decimals = 6,
    initialSupply = new BN(1_000_000),
    name = "",
    symbol = "",
    uri = "",
    user = fixture.payer,
    treasury = fixture.treasury.publicKey,
    oracleProgram = ORACLE_PROGRAM_ID,
    oracleState = fixture.oraclePda,
    tokenMetadataProgram = SystemProgram.programId,
    metadata,
  }: {
    decimals?: number;
    initialSupply?: BN;
    name?: string;
    symbol?: string;
    uri?: string;
    user?: Keypair;
    treasury?: PublicKey;
    oracleProgram?: PublicKey;
    oracleState?: PublicKey;
    tokenMetadataProgram?: PublicKey;
    metadata?: PublicKey;
  } = {}
) {
  const mintKeypair = Keypair.generate();
  const userAta = anchor.utils.token.associatedAddress({
    owner: user.publicKey,
    mint: mintKeypair.publicKey,
  });
  const [metadataPda] = PublicKey.findProgramAddressSync(
    [
      METADATA_SEED,
      tokenMetadataProgram.toBytes(),
      mintKeypair.publicKey.toBytes(),
    ],
    tokenMetadataProgram
  );

  const ix = new TransactionInstruction({
    programId: MINTER_PROGRAM_ID,
    keys: [
      { pubkey: fixture.minterPda, isSigner: false, isWritable: true },
      { pubkey: user.publicKey, isSigner: true, isWritable: true },
      { pubkey: treasury, isSigner: false, isWritable: true },
      { pubkey: oracleProgram, isSigner: false, isWritable: false },
      { pubkey: oracleState, isSigner: false, isWritable: false },
      { pubkey: mintKeypair.publicKey, isSigner: true, isWritable: true },
      { pubkey: userAta, isSigner: false, isWritable: true },
      { pubkey: tokenMetadataProgram, isSigner: false, isWritable: false },
      { pubkey: metadata ?? metadataPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      {
        pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      {
        pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
        isSigner: false,
        isWritable: false,
      },
    ],
    data: minterCoder.encode("mint_token", {
      decimals,
      initial_supply: initialSupply,
      name,
      symbol,
      uri,
    }),
  });
  (ix as any)._additionalSigners = [mintKeypair];

  return { ix, mintKeypair, metadataPda, userAta };
}

describe("token_minter (LiteSVM)", () => {
  const svm = new LiteSVM()
    .withSysvars()
    .withBuiltins()
    .withDefaultPrograms()
    .withBlockhashCheck(false)
    .withSigverify(false);
  svm.addProgramFromFile(ORACLE_PROGRAM_ID, ORACLE_SO);
  svm.addProgramFromFile(MINTER_PROGRAM_ID, MINTER_SO);

  const payer = Keypair.generate();
  const treasury = Keypair.generate();
  const [oraclePda] = PublicKey.findProgramAddressSync(
    [ORACLE_SEED],
    ORACLE_PROGRAM_ID
  );
  const [minterPda] = PublicKey.findProgramAddressSync(
    [MINTER_SEED],
    MINTER_PROGRAM_ID
  );

  before(() => {
    svm.airdrop(payer.publicKey, BigInt(10_000_000_000));
    svm.airdrop(treasury.publicKey, BigInt(1_000_000_000));
  });

  it("initialize oracle + minter and mint token with fee", () => {
    // init oracle
    const initOracleIx = new TransactionInstruction({
      programId: ORACLE_PROGRAM_ID,
      keys: [
        { pubkey: oraclePda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: oracleCoder.encode("initialize_oracle", { admin: payer.publicKey }),
    });

    // update price
    const updateIx = new TransactionInstruction({
      programId: ORACLE_PROGRAM_ID,
      keys: [
        { pubkey: oraclePda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      ],
      data: oracleCoder.encode("update_price", { new_price: PRICE }),
    });

    // init minter
    const initMinterIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: minterPda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: minterCoder.encode("initialize_minter", {
        treasury: treasury.publicKey,
        mint_fee_usd: FEE_USD,
        oracle_state: oraclePda,
        oracle_program: ORACLE_PROGRAM_ID,
      }),
    });

    const mintKeypair = Keypair.generate();
    const user = payer;
    const userAta = anchor.utils.token.associatedAddress({
      owner: user.publicKey,
      mint: mintKeypair.publicKey,
    });

    const mintIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: minterPda, isSigner: false, isWritable: true },
        { pubkey: user.publicKey, isSigner: true, isWritable: true },
        { pubkey: treasury.publicKey, isSigner: false, isWritable: true },
        { pubkey: ORACLE_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: oraclePda, isSigner: false, isWritable: false },
        { pubkey: mintKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: userAta, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        {
          pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
          pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
          isSigner: false,
          isWritable: false,
        },
      ],
      data: minterCoder.encode("mint_token", {
        decimals: 6,
        initial_supply: new BN(1_000_000),
        name: "",
        symbol: "",
        uri: "",
      }),
    });
    // attach extra signer for mint
    (mintIx as any)._additionalSigners = [mintKeypair];

    // send setup txs
    for (const ix of [initOracleIx, updateIx, initMinterIx]) {
      const res = svm.sendTransaction(buildTx([ix], payer, svm));
      assertSuccess(res);
    }

    const treasuryBefore = svm.getBalance(treasury.publicKey) ?? BigInt(0);
    const res = svm.sendTransaction(buildTx([mintIx], payer, svm));
    assertSuccess(res);

    const treasuryAfter = svm.getBalance(treasury.publicKey) ?? BigInt(0);
    const expectedFee = FEE_USD.mul(new BN(anchor.web3.LAMPORTS_PER_SOL)).div(
      PRICE
    );
    expect(treasuryAfter - treasuryBefore).to.eq(
      BigInt(expectedFee.toString())
    );

    const mintAcct = svm.getAccount(mintKeypair.publicKey);
    expect(mintAcct).to.not.be.null;

    const ataAcct = svm.getAccount(userAta);
    expect(ataAcct).to.not.be.null;

    const cfgRaw = svm.getAccount(minterPda);
    const cfg: any = minterAccounts.decode(
      "MinterConfig",
      Buffer.from((cfgRaw as any).data)
    );
    const mintFee = cfg.mint_fee_usd ?? cfg.mintFeeUsd;
    expect(mintFee.toString()).to.eq(FEE_USD.toString());
    const treasuryPk: PublicKey =
      cfg.treasury ?? cfg.treasuryPubkey ?? cfg.treasury_pubkey;
    expect(treasuryPk.toBase58()).to.eq(treasury.publicKey.toBase58());

    const oracleRaw = svm.getAccount(oraclePda);
    const oracle = oracleAccounts.decode(
      "OracleState",
      Buffer.from((oracleRaw as any).data)
    );
    expect(oracle.price.toString()).to.eq(PRICE.toString());
  });

  it("rejects mint when initial supply is zero", () => {
    const mintKeypair = Keypair.generate();
    const userAta = anchor.utils.token.associatedAddress({
      owner: payer.publicKey,
      mint: mintKeypair.publicKey,
    });
    const mintIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: minterPda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: treasury.publicKey, isSigner: false, isWritable: true },
        { pubkey: ORACLE_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: oraclePda, isSigner: false, isWritable: false },
        { pubkey: mintKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: userAta, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        {
          pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
          pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
          isSigner: false,
          isWritable: false,
        },
      ],
      data: minterCoder.encode("mint_token", {
        decimals: 6,
        initial_supply: new BN(0),
        name: "",
        symbol: "",
        uri: "",
      }),
    });
    (mintIx as any)._additionalSigners = [mintKeypair];

    const treasuryBefore = svm.getBalance(treasury.publicKey) ?? BigInt(0);
    const res = svm.sendTransaction(buildTx([mintIx], payer, svm));
    assertFailure(res);
    const treasuryAfter = svm.getBalance(treasury.publicKey) ?? BigInt(0);
    expect(treasuryAfter).to.eq(treasuryBefore);
  });

  it("rejects mint when decimals exceed allowed range", () => {
    const mintKeypair = Keypair.generate();
    const userAta = anchor.utils.token.associatedAddress({
      owner: payer.publicKey,
      mint: mintKeypair.publicKey,
    });
    const mintIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: minterPda, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: treasury.publicKey, isSigner: false, isWritable: true },
        { pubkey: ORACLE_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: oraclePda, isSigner: false, isWritable: false },
        { pubkey: mintKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: userAta, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        {
          pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        {
          pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
          isSigner: false,
          isWritable: false,
        },
      ],
      data: minterCoder.encode("mint_token", {
        decimals: 10,
        initial_supply: new BN(1_000_000),
        name: "",
        symbol: "",
        uri: "",
      }),
    });
    (mintIx as any)._additionalSigners = [mintKeypair];

    const treasuryBefore = svm.getBalance(treasury.publicKey) ?? BigInt(0);
    const res = svm.sendTransaction(buildTx([mintIx], payer, svm));
    assertFailure(res);
    const treasuryAfter = svm.getBalance(treasury.publicKey) ?? BigInt(0);
    expect(treasuryAfter).to.eq(treasuryBefore);
  });
});

describe("token_minter edge cases (LiteSVM)", () => {
  it("allows only admin to update fee and treasury", () => {
    const fixture = createMinterFixture();
    initializeOracleAndMinter(fixture);
    const attacker = Keypair.generate();
    const nextTreasury = Keypair.generate();
    fixture.svm.airdrop(attacker.publicKey, BigInt(1_000_000_000));

    const nonAdminFeeIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: fixture.minterPda, isSigner: false, isWritable: true },
        { pubkey: attacker.publicKey, isSigner: true, isWritable: false },
      ],
      data: minterCoder.encode("set_fee_usd", {
        new_fee_usd: new BN(7_000_000),
      }),
    });
    assertFailure(sendFixtureIx(fixture, nonAdminFeeIx, attacker));

    const zeroFeeIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: fixture.minterPda, isSigner: false, isWritable: true },
        { pubkey: fixture.payer.publicKey, isSigner: true, isWritable: false },
      ],
      data: minterCoder.encode("set_fee_usd", { new_fee_usd: new BN(0) }),
    });
    assertFailure(sendFixtureIx(fixture, zeroFeeIx));

    const adminFeeIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: fixture.minterPda, isSigner: false, isWritable: true },
        { pubkey: fixture.payer.publicKey, isSigner: true, isWritable: false },
      ],
      data: minterCoder.encode("set_fee_usd", {
        new_fee_usd: new BN(7_000_000),
      }),
    });
    assertSuccess(sendFixtureIx(fixture, adminFeeIx));

    const nonAdminTreasuryIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: fixture.minterPda, isSigner: false, isWritable: true },
        { pubkey: attacker.publicKey, isSigner: true, isWritable: false },
      ],
      data: minterCoder.encode("set_treasury", {
        new_treasury: attacker.publicKey,
      }),
    });
    assertFailure(sendFixtureIx(fixture, nonAdminTreasuryIx, attacker));

    const adminTreasuryIx = new TransactionInstruction({
      programId: MINTER_PROGRAM_ID,
      keys: [
        { pubkey: fixture.minterPda, isSigner: false, isWritable: true },
        { pubkey: fixture.payer.publicKey, isSigner: true, isWritable: false },
      ],
      data: minterCoder.encode("set_treasury", {
        new_treasury: nextTreasury.publicKey,
      }),
    });
    assertSuccess(sendFixtureIx(fixture, adminTreasuryIx));

    const cfgRaw = fixture.svm.getAccount(fixture.minterPda);
    const cfg: any = minterAccounts.decode(
      "MinterConfig",
      Buffer.from((cfgRaw as any).data)
    );
    const mintFee = cfg.mint_fee_usd ?? cfg.mintFeeUsd;
    expect(mintFee.toString()).to.eq("7000000");
    expect(cfg.treasury.toBase58()).to.eq(nextTreasury.publicKey.toBase58());
  });

  it("rejects mint when oracle price is still zero", () => {
    const fixture = createMinterFixture();
    initializeOracleAndMinter(fixture, { price: null });

    const { ix } = buildMintInstruction(fixture);
    const treasuryBefore =
      fixture.svm.getBalance(fixture.treasury.publicKey) ?? BigInt(0);
    assertFailure(sendFixtureIx(fixture, ix));
    const treasuryAfter =
      fixture.svm.getBalance(fixture.treasury.publicKey) ?? BigInt(0);
    expect(treasuryAfter).to.eq(treasuryBefore);
  });

  it("rejects mint when provided treasury does not match config", () => {
    const fixture = createMinterFixture();
    initializeOracleAndMinter(fixture);
    const wrongTreasury = Keypair.generate();
    fixture.svm.airdrop(wrongTreasury.publicKey, BigInt(1_000_000_000));

    const { ix } = buildMintInstruction(fixture, {
      treasury: wrongTreasury.publicKey,
    });
    assertFailure(sendFixtureIx(fixture, ix));
  });

  it("rejects mint when configured oracle state differs from the provided account", () => {
    const fixture = createMinterFixture();
    const wrongOracleState = Keypair.generate().publicKey;
    initializeOracleAndMinter(fixture, { oracleState: wrongOracleState });

    const { ix } = buildMintInstruction(fixture);
    assertFailure(sendFixtureIx(fixture, ix));
  });

  it("rejects mint when fee calculation overflows", () => {
    const fixture = createMinterFixture();
    initializeOracleAndMinter(fixture, {
      price: new BN(1),
      mintFeeUsd: U64_MAX,
    });

    const { ix } = buildMintInstruction(fixture);
    assertFailure(sendFixtureIx(fixture, ix));
  });

  it("rejects metadata mint when metadata program or PDA is invalid", () => {
    const fixture = createMinterFixture();
    initializeOracleAndMinter(fixture);

    const invalidProgramMint = buildMintInstruction(fixture, {
      name: "Test token",
      symbol: "TST",
      uri: "https://example.com/token.json",
      tokenMetadataProgram: SystemProgram.programId,
    });
    assertFailure(sendFixtureIx(fixture, invalidProgramMint.ix));

    const invalidPdaMint = buildMintInstruction(fixture, {
      name: "Test token",
      symbol: "TST",
      uri: "https://example.com/token.json",
      tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
      metadata: Keypair.generate().publicKey,
    });
    assertFailure(sendFixtureIx(fixture, invalidPdaMint.ix));
  });
});
