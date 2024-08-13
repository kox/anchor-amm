import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorAmm } from "../target/types/anchor_amm";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { randomBytes } from "crypto"
import { BN } from "bn.js";
import { assert } from "chai";

import { newMintToAta } from './utils';
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";


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

  it("should airdrop SOL to the main users!", async () => {
    let tx = new Transaction();
    tx.instructions = [
      ...[creatorPool, userPool].map((account) =>

        SystemProgram.transfer({
          fromPubkey: provider.publicKey,
          toPubkey: account.publicKey,
          lamports: 10 * LAMPORTS_PER_SOL,
        })
      )];
    await provider.sendAndConfirm(tx).then(log);

    const creatorBalance = await connection.getBalance(creatorPool.publicKey);
    const userBalance = await connection.getBalance(userPool.publicKey);
    assert.equal(creatorBalance, 10 * LAMPORTS_PER_SOL);
    assert.equal(userBalance, 10 * LAMPORTS_PER_SOL);
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
    assert.equal(xAtaBalance.value.uiAmount, 1000, "xAta should have 1000 tokens");
    assert.equal(yAtaBalance.value.uiAmount, 1000, "yAta should have 1000 tokens");

    // Assert that vault ATAs are initialized with zero balance
    const xVaultAtaBalance = await connection.getTokenAccountBalance(xVaultAta);
    const yVaultAtaBalance = await connection.getTokenAccountBalance(yVaultAta);
    assert.equal(xVaultAtaBalance.value.uiAmount, 0, "xVaultAta should have 0 tokens initially");
    assert.equal(yVaultAtaBalance.value.uiAmount, 0, "yVaultAta should have 0 tokens initially");

  });
});
