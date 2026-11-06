import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddressSync,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { assert } from "chai";
import { LiquidityVault } from "../target/types/liquidity_vault";

describe("liquidity_vault (week 8 skeleton)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.LiquidityVault as Program<LiquidityVault>;
  const admin = provider.wallet as anchor.Wallet;
  const payer = admin.payer;

  let underlyingMint: PublicKey;
  let config: PublicKey;
  let shareMint: PublicKey;
  let vaultTokenAccount: PublicKey;
  let userPosition: PublicKey;
  let userTokenAccount: PublicKey;
  let userShareAccount: PublicKey;

  const DECIMALS = 6;
  const ONE = 10 ** DECIMALS;

  before(async () => {
    underlyingMint = await createMint(provider.connection, payer, admin.publicKey, null, DECIMALS);

    [config] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), underlyingMint.toBuffer()],
      program.programId
    );
    [shareMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("shares"), config.toBuffer()],
      program.programId
    );
    [vaultTokenAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), config.toBuffer()],
      program.programId
    );
    [userPosition] = PublicKey.findProgramAddressSync(
      [Buffer.from("user"), config.toBuffer(), admin.publicKey.toBuffer()],
      program.programId
    );

    userTokenAccount = (
      await getOrCreateAssociatedTokenAccount(provider.connection, payer, underlyingMint, admin.publicKey)
    ).address;
    await mintTo(provider.connection, payer, underlyingMint, userTokenAccount, payer, 1_000 * ONE);
    userShareAccount = getAssociatedTokenAddressSync(shareMint, admin.publicKey);
  });

  it("initializes vault: config + share mint + vault token account", async () => {
    await program.methods
      .initializeVault(50, new BN(500 * ONE)) // 0.5% fee, cap 500 tokens
      .accountsPartial({
        admin: admin.publicKey,
        underlyingMint,
        config,
        shareMint,
        vaultTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const cfg = await program.account.vaultConfig.fetch(config);
    assert.ok(cfg.admin.equals(admin.publicKey));
    assert.ok(cfg.shareMint.equals(shareMint));
    assert.equal(cfg.feeBps, 50);
    assert.equal(cfg.paused, false);
  });

  it("creates the user PDA", async () => {
    await program.methods
      .initializeUser()
      .accountsPartial({ owner: admin.publicKey, config, userPosition })
      .rpc();

    const pos = await program.account.userPosition.fetch(userPosition);
    assert.ok(pos.owner.equals(admin.publicKey));
    assert.equal(pos.shares.toNumber(), 0);
  });

  it("deposits and mints shares 1:1 on first deposit", async () => {
    await program.methods
      .deposit(new BN(100 * ONE))
      .accountsPartial({
        owner: admin.publicKey,
        config,
        underlyingMint,
        shareMint,
        vaultTokenAccount,
        userPosition,
        userTokenAccount,
        userShareAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const shares = await getAccount(provider.connection, userShareAccount);
    const vault = await getAccount(provider.connection, vaultTokenAccount);
    assert.equal(Number(shares.amount), 100 * ONE);
    assert.equal(Number(vault.amount), 100 * ONE);
    const pos = await program.account.userPosition.fetch(userPosition);
    assert.equal(pos.shares.toNumber(), 100 * ONE);
  });

  it("rejects deposit above the cap", async () => {
    try {
      await program.methods
        .deposit(new BN(600 * ONE))
        .accountsPartial({
          owner: admin.publicKey,
          config,
          underlyingMint,
          shareMint,
          vaultTokenAccount,
          userPosition,
          userTokenAccount,
          userShareAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      assert.fail("should have failed");
    } catch (e: any) {
      assert.include(e.toString(), "DepositCapExceeded");
    }
  });

  it("withdraws: burns shares, returns tokens minus fee", async () => {
    await program.methods
      .withdraw(new BN(40 * ONE))
      .accountsPartial({
        owner: admin.publicKey,
        config,
        underlyingMint,
        shareMint,
        vaultTokenAccount,
        userPosition,
        userTokenAccount,
        userShareAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const shares = await getAccount(provider.connection, userShareAccount);
    assert.equal(Number(shares.amount), 60 * ONE);
    const vault = await getAccount(provider.connection, vaultTokenAccount);
    // 40 * 0.5% = 0.2 fee stays in the vault
    assert.equal(Number(vault.amount), 60 * ONE + 0.2 * ONE);
  });

  it("admin can pause; deposits are blocked while paused", async () => {
    await program.methods
      .updateConfig(null, null, true)
      .accountsPartial({ admin: admin.publicKey, config })
      .rpc();

    try {
      await program.methods
        .deposit(new BN(1 * ONE))
        .accountsPartial({
          owner: admin.publicKey,
          config,
          underlyingMint,
          shareMint,
          vaultTokenAccount,
          userPosition,
          userTokenAccount,
          userShareAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      assert.fail("should have failed");
    } catch (e: any) {
      assert.include(e.toString(), "VaultPaused");
    }
  });
});
