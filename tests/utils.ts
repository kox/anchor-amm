import { getProvider } from "@coral-xyz/anchor";
import { createAccount, createMint, mintTo } from "@solana/spl-token";
import { Commitment, Connection, Keypair, PublicKey } from "@solana/web3.js";

export const commitment: Commitment = "confirmed"; // processed, confirmed, finalized


export function delay(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

const confirmTx = async (connection: Connection, signature: string) => {
  const latestBlockhash = await connection.getLatestBlockhash();
  await connection.confirmTransaction(
    {
      signature,
      ...latestBlockhash,
    },
    commitment
  )
}

const confirmTxs = async (connection, signatures: string[]) => {
  await Promise.all(signatures.map(connection, confirmTx))
}

export interface INewMintToAta {
  mint: PublicKey;
  ata: PublicKey;
};

export const newMintToAta = async (connection, minter: Keypair): Promise<INewMintToAta> => {
  const mint = await createMint(connection, minter, minter.publicKey, null, 6)
  
  // await getAccount(connection, mint, commitment)
  const ata = await createAccount(connection, minter, mint, minter.publicKey)
  const signature = await mintTo(connection, minter, mint, ata, minter, 1e9)
  
  await confirmTx(connection, signature)

  return {
    mint,
    ata
  }
}