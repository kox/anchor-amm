import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorAmm } from "../target/types/anchor_amm";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { randomBytes } from "crypto"
import { BN } from "bn.js";
import { assert, expect } from "chai";

import { commitment, newMintToAta } from './utils';
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAccount, getAssociatedTokenAddress, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";


describe("anchor-amm", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();

  const connection = provider.connection;

  const program = anchor.workspace.AnchorAmm as Program<AnchorAmm>;

  // We will use these two helpers to log and wait for confirmation
  const confirm = async (signature: string): Promise<string> => {
    const block = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...block,
    });
    return signature;
  };

  const log = async (signature: string): Promise<string> => {
    console.log(
      `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=${connection.rpcEndpoint}`
    );
    return signature;
  };

  // We create two new keypairs who will make use of the AMM
  const [creatorPool, userPool] = [new Keypair(), new Keypair()];

  // To make the pool unique, we will generate a unique seed
  const seed = new BN(randomBytes(8));

  // PDAs
  const [auth, authBump] = PublicKey.findProgramAddressSync([
    Buffer.from("auth")],
    program.programId
  );
  const [config, configBump] = PublicKey.findProgramAddressSync([
    Buffer.from("config"),
    seed.toBuffer().reverse()
  ], program.programId
  );

  // Three different mints. X, Y and LP. X and Y will get created during the tests and LP can be derived
  let xMint: PublicKey = undefined;
  let yMint: PublicKey = undefined;
  let [lpMint, lpMintBump] = PublicKey.findProgramAddressSync([
    Buffer.from("lp"),
    config.toBuffer()
  ], program.programId);

  // ATAs to store the tokens
  let xAta: PublicKey = undefined;
  let yAta: PublicKey = undefined;
  let lpAta: PublicKey = undefined;
  let xVaultAta: PublicKey = undefined;
  let yVaultAta: PublicKey = undefined;
  let lpVaultAta: PublicKey = undefined;

  const mintNumber = 2e9;
  const depositNumber = 1e9;
  const expiration = new BN(Math.floor(new Date().getTime() / 1000) + 600);

  const accounts = {
    auth,
    xMint,
    yMint,
    lpMint,
    xVaultAta,
    yVaultAta,
    config,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    SystemProgram: SystemProgram.programId,
  }

  it("should airdrop SOL to the main users!", async () => {
    let tx = new Transaction();
    tx.instructions = [
      ...[creatorPool, userPool].map((account) =>

        SystemProgram.transfer({
          fromPubkey: provider.publicKey,
          toPubkey: account.publicKey,
          lamports: 20 * LAMPORTS_PER_SOL,
        })
      )];
    await provider.sendAndConfirm(tx).then(log);

    const creatorBalance = await connection.getBalance(creatorPool.publicKey);
    const userBalance = await connection.getBalance(userPool.publicKey);
    assert.equal(creatorBalance, 20 * LAMPORTS_PER_SOL);
    assert.equal(userBalance, 20 * LAMPORTS_PER_SOL);
  });

  it('should create all mints and atas required for starting the AMM', async () => {
    let [xToken, yToken] = await Promise.all(
      [creatorPool, creatorPool].map(async (user) => await newMintToAta(connection, user)
      ));

    xMint = xToken.mint;
    yMint = yToken.mint;

    xAta = xToken.ata;
    yAta = yToken.ata;
    lpAta = await getAssociatedTokenAddress(lpMint, creatorPool.publicKey, false, TOKEN_PROGRAM_ID);

    // vault atas
    xVaultAta = await getAssociatedTokenAddress(xMint, auth, true, TOKEN_PROGRAM_ID);
    yVaultAta = await getAssociatedTokenAddress(yMint, auth, true, TOKEN_PROGRAM_ID);
    lpVaultAta = await getAssociatedTokenAddress(lpMint, auth, true, TOKEN_PROGRAM_ID);

    // Assert that mints are defined
    assert.exists(xMint, "xMint should be created");
    assert.exists(yMint, "yMint should be created");

    // Assert that ATAs are defined
    assert.exists(xAta, "xAta should be created");
    assert.exists(yAta, "yAta should be created");
    assert.exists(lpAta, "lpAta should be created");
    assert.exists(xVaultAta, "xVaultAta should be created");
    assert.exists(yVaultAta, "yVaultAta should be created");
    assert.exists(lpVaultAta, "lpVaultAta should be created");

    // Assert that ATAs belong to the correct owner
    const xAtaInfo = await connection.getParsedAccountInfo(xAta);
    const yAtaInfo = await connection.getParsedAccountInfo(yAta);
    assert.equal((xAtaInfo.value.data as any).parsed.info.owner, creatorPool.publicKey.toBase58(), "xAta should belong to creatorPool");
    assert.equal((yAtaInfo.value.data as any).parsed.info.owner, creatorPool.publicKey.toBase58(), "yAta should belong to creatorPool");

    // Optionally, assert the initial token balances (assuming newMintToAta mints some tokens)
    const xAtaBalance = await connection.getTokenAccountBalance(xAta);
    const yAtaBalance = await connection.getTokenAccountBalance(yAta);

    assert.equal(xAtaBalance.value.uiAmount, 2000, "xAta should have 2000 tokens");
    assert.equal(yAtaBalance.value.uiAmount, 2000, "yAta should have 2000 tokens");
  });

  it('should initialize the config account and the 2 empty vaults per X and Y tokens', async () => {
    await program.methods.initialize(seed, 0, creatorPool.publicKey)
      .accounts({
        payer: creatorPool.publicKey,
        xMint,
        yMint,
        /* xVault: xVaultAta,
        yVault: yVaultAta,  */
      })
      .signers([creatorPool])
      .rpc()
      .then(confirm)
      .then(log);

    // Assert that vault ATAs are initialized with zero balance
    const xVaultAtaBalance = await connection.getTokenAccountBalance(xVaultAta);
    const yVaultAtaBalance = await connection.getTokenAccountBalance(yVaultAta);

    assert.equal(xVaultAtaBalance.value.uiAmount, 0, "xVaultAta should have 0 tokens initially");
    assert.equal(yVaultAtaBalance.value.uiAmount, 0, "yVaultAta should have 0 tokens initially");

    // The config fee should be fillup too
    const configAccount = await program.account.config.fetch(config);
    assert.equal(configAccount.seed.toString(), seed.toString());
    assert.equal(configAccount.authority.toString(), creatorPool.publicKey.toString());
    assert.equal(configAccount.fee, 0);
    assert.equal(configAccount.locked, false);
  });

  it('should not allow to modify config account without authority', async () => {
    try {
      await program.methods.lock()
      .accounts({
        payer: userPool.publicKey,
        config,
      })
      .signers([userPool])
      .rpc()
      .then(confirm)
      .then(log);
    } catch (err) {
      assert.equal(err.error.errorCode.code, "InvalidAuthority");
      assert.equal(err.error.errorMessage, "Invalid update authority.")
    }
  });

  it('should lock the config and don\'t allow deposits', async () => {
    await program.methods.lock()
      .accounts({
        payer: creatorPool.publicKey,
        config,
      })
      .signers([creatorPool])
      .rpc()
      .then(confirm)
      .then(log);

    // The pool should be locked
    const configAccount = await program.account.config.fetch(config);
    assert.equal(configAccount.locked, true);

    try {
      await program.methods.deposit(
        new BN(1 * 1e6),
        new BN(depositNumber),
        new BN(depositNumber),
        expiration,
      )
        .accounts({
          payer: creatorPool.publicKey,
          config: config,
          xMint: xMint,
          yMint: yMint,
          xVaultAta: xVaultAta,
          yVaultAta: yVaultAta,
          xUser: xAta,
          yUser: yAta,
        })
        .signers([creatorPool])
        .rpc();

        throw Error("should not arrive here!");
      } catch (err) {
        assert.equal(err.error.errorCode.code, "PoolLocked");
        assert.equal(err.error.errorMessage, "This pool is locked.")
      }
  });

  it('should lock the config and don\'t allow deposits', async () => {
    await program.methods.unlock()
      .accounts({
        payer: creatorPool.publicKey,
        config,
      })
      .signers([creatorPool])
      .rpc()
      .then(confirm)
      .then(log);

    // The pool should be locked
    const configAccount = await program.account.config.fetch(config);
    assert.equal(configAccount.locked, false);
  });

  it('should be able to deposity tokens to the LP and it will receive LP tokens', async () => {
    // Assert that vault ATAs are initialized with zero balance
    const xVaultAtaBalance = await connection.getTokenAccountBalance(xVaultAta);
    const yVaultAtaBalance = await connection.getTokenAccountBalance(yVaultAta);

    assert.equal(xVaultAtaBalance.value.uiAmount, 0, "xVaultAta should have 0 tokens initially");
    assert.equal(yVaultAtaBalance.value.uiAmount, 0, "yVaultAta should have 0 tokens initially");

    await program.methods.deposit(
      new BN(1 * 1e6),
      new BN(depositNumber),
      new BN(depositNumber),
      expiration,
    )
      .accounts({
        payer: creatorPool.publicKey,
        config: config,
        xMint: xMint,
        yMint: yMint,
        xVaultAta: xVaultAta,
        yVaultAta: yVaultAta,
        xUser: xAta,
        yUser: yAta,
      })
      .signers([creatorPool])
      .rpc()
      .then(confirm)
      .then(log);


    // Assert that vault ATAs are initialized with zero balance
    const xBalance = await connection.getTokenAccountBalance(xVaultAta);
    const yBalance = await connection.getTokenAccountBalance(yVaultAta);

    assert.equal(xBalance.value.amount, yBalance.value.amount, "Vaults are unbalanced");
    assert.equal(xBalance.value.amount, depositNumber.toString(), "Vaults didn't get the correct deposit");


    // Optionally, assert the initial token balances (assuming newMintToAta mints some tokens)
    const xAtaBalance = await connection.getTokenAccountBalance(xAta);
    const yAtaBalance = await connection.getTokenAccountBalance(yAta);

    assert.equal(xAtaBalance.value.amount, (mintNumber - depositNumber).toString(), "Wrong number of X tokens left in the creator");
    assert.equal(xAtaBalance.value.amount, (mintNumber - depositNumber).toString(), "Wrong number of Y tokens left in the creator");

    // We need to check the pool minted those lp tokens
    const lpAtaBalance = await connection.getTokenAccountBalance(lpAta);
    assert.equal(lpAtaBalance.value.amount, 1e6.toString());
  });

  
  it('should not be possible to swap 0 tokens', async () => {
    try {
      await program.methods.swap(
        new BN(0),
        new BN(1e5),
        true,
      )
        .accounts({
          payer: creatorPool.publicKey,
          config: config,
          xMint: xMint,
          yMint: yMint,
          xVaultAta: xVaultAta,
          yVaultAta: yVaultAta,
          xUser: xAta,
          yUser: yAta,
        })
        .signers([creatorPool])
        .rpc();

        throw Error("IT should not arrive to this point.");
    } catch (err) {
      assert.ok(true);
    }
    
  });


  it('should be able to swap some X tokens', async () => {
    await program.methods.swap(
      new BN(1e6),
      new BN(1e5),
      true,
      expiration,
    )
      .accounts({
        payer: creatorPool.publicKey,
        config: config,
        xMint: xMint,
        yMint: yMint,
        xVaultAta: xVaultAta,
        yVaultAta: yVaultAta,
        xUser: xAta,
        yUser: yAta,
      })
      .signers([creatorPool])
      .rpc()
      .then(confirm)
      .then(log);

      // Assert that vault ATAs are initialized with zero balance
      const xBalance = await connection.getTokenAccountBalance(xVaultAta);
      const yBalance = await connection.getTokenAccountBalance(yVaultAta);

      assert.equal(xBalance.value.amount.toString(), "1000000999");
      assert.equal(yBalance.value.amount.toString(), "1000000000");

      // Optionally, assert the initial token balances (assuming newMintToAta mints some tokens)
      const xAtaBalance = await connection.getTokenAccountBalance(xAta);
      const yAtaBalance = await connection.getTokenAccountBalance(yAta);

      assert.equal(xAtaBalance.value.amount.toString(), "999999001");
      assert.equal(yAtaBalance.value.amount.toString(), "1000000000");
  });  

});

