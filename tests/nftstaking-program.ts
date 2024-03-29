import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftStakingProgram } from "../target/types/nft_staking_program";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram, 
  SYSVAR_RENT_PUBKEY 
} from '@solana/web3.js';
import { safeAirdrop, delay } from './utils/utils'
import { PROGRAM_ID as METADATA_PROGRAM_ID } from '@metaplex-foundation/mpl-token-metadata';
import { 
  TOKEN_PROGRAM_ID, 
  getAssociatedTokenAddress, 
  ASSOCIATED_TOKEN_PROGRAM_ID, 
  getAccount
} from '@solana/spl-token'
import { expect } from "chai";
describe("nft-staking-program", async () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  // Access the NftStakingProgram instance from the Anchor workspace.
  const program = anchor.workspace.NftStakingProgram as Program<NftStakingProgram>;
  const provider = anchor.AnchorProvider.env();

  // Generate a new keypair for the NFT mint and user.
  const nftMint = Keypair.generate();
  const user = Keypair.generate();

  // Get the associated token account for the user.
  const userTokenAccount = await getAssociatedTokenAddress(
    nftMint.publicKey,
    user.publicKey
  );

  // Find the metadata and master edition public keys.
  const [metadata, metadataBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), 
    METADATA_PROGRAM_ID.toBuffer(), 
    nftMint.publicKey.toBuffer()],
    METADATA_PROGRAM_ID
  );

  const [masterEdition, masterBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), 
    METADATA_PROGRAM_ID.toBuffer(), 
    nftMint.publicKey.toBuffer(), 
    Buffer.from("edition")],
    METADATA_PROGRAM_ID
  );

  // Find the stake public key.
  const stake = PublicKey.findProgramAddressSync(
    [user.publicKey.toBuffer(), 
      userTokenAccount.toBuffer()],
    program.programId
  );

  // Find the program authority, token mint, and mint authority public keys.
  const programAuthority = PublicKey.findProgramAddressSync(
    [Buffer.from("authority")],
    program.programId
  );

  const tokenMint = PublicKey.findProgramAddressSync(
    [Buffer.from("token-mint")],
    program.programId
  );

  const mintAuthority = PublicKey.findProgramAddressSync(
    [Buffer.from("mint-authority")],
    program.programId
  );

  // Get the associated reward token account.
  const rewardTokenAccount = await getAssociatedTokenAddress(
    tokenMint[0],
    user.publicKey
  );

  // Create and mint NFT test case
  it("Create and mint NFT!", async () => {
    // Safe airdrop some tokens to the user
    await safeAirdrop(user.publicKey, provider.connection);

    const name = "Uhanmi NFT";
    const symbol = "UNFT";
    const uri = "www.uhanmiuri.com";

    // Create and mint the NFT using the program's method
    const txid = await program.methods.createNft(name, symbol, uri)
      .accounts({
        user: user.publicKey,
        userTokenAccount: userTokenAccount,
        nftMint: nftMint.publicKey,
        metadataAccount: metadata,
        masterEdition: masterEdition,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        metadataProgram: METADATA_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([user, nftMint])
      .rpc();

    // Print transaction details for viewing
    console.log("View transaction in explorer:");
    console.log(`https://explorer.solana.com/tx/${txid}?cluster=devnet`);
    console.log("View NFT in explorer:");
    console.log(`https://explorer.solana.com/address/${nftMint.publicKey}?cluster=devnet`);
  });

  // Stake NFT test case
  it("Stake NFT!", async () => {
    // Stake the NFT using the program's method
    const txid = await program.methods.stake()
      .accounts({
        user: user.publicKey,
        userTokenAccount: userTokenAccount,
        nftMint: nftMint.publicKey,
        stake: stake[0],
        masterEdition: masterEdition,
        tokenProgram: TOKEN_PROGRAM_ID,
        metadataProgram: METADATA_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
        programAuthority: programAuthority[0],
      })
      .signers([user])
      .rpc();

    // Print transaction details for viewing
    console.log("View staking transaction in explorer:");
    console.log(`https://explorer.solana.com/tx/${txid}?cluster=devnet`);

    // Fetch the user token account info
    const tokenAccountInfo = await getAccount(
      provider.connection,
      userTokenAccount
    );

    // Validate token account details
    expect(tokenAccountInfo.isFrozen).to.be.true;
    expect(tokenAccountInfo.amount).to.equal(BigInt(1));
  });

  // Initialize mint test case (skipped)
  it.skip("Initialize mint.", async () => {
    // Initialize mint using the program's method (skipped in this case)
  });

  // Unstake NFT test case
  it("Unstake NFT.", async () => {
    // Wait for at least 1s to get at least one reward token
    await delay(1000);

    // Unstake the NFT using the program's method
    const txid = await program.methods.unstake()
      .accounts({
        user: user.publicKey,
        nftMint: nftMint.publicKey,
        stake: stake[0],
        nftTokenAccount: userTokenAccount,
        masterEdition: masterEdition,
        programAuthority: programAuthority[0],
        tokenMint: tokenMint[0],
        mintAuthority: mintAuthority[0],
        userTokenAccount: rewardTokenAccount,
        metadataProgram: METADATA_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([user])
      .rpc();

    // Print transaction details for viewing
    console.log("View unstake transaction in explorer:");
    console.log(`https://explorer.solana.com/tx/${txid}?cluster=devnet`);

    // Fetch the NFT token account info
    const nftAccountInfo = await getAccount(
      provider.connection,
      userTokenAccount
    );

    // Validate NFT token account details
    expect(nftAccountInfo.isFrozen).to.be.false;
    expect(nftAccountInfo.delegate).to.be.null;

    // Fetch the reward token account info
    const tokenAccountInfo = await getAccount(
      provider.connection,
      rewardTokenAccount
    );

    // Validate reward token account details
    expect(Number.parseInt(tokenAccountInfo.amount.toString())).to.be.greaterThanOrEqual(1);
  });
});

