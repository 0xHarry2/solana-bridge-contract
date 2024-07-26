import * as anchor from "@coral-xyz/anchor";
import * as web3 from "@solana/web3.js";
import splToken from "@solana/spl-token";

import idl from "../target/idl/solana_cdt_bridge.json";
import PrivateKey from "./privateKey.json";

const programId = new web3.PublicKey( // Bridge program id from the deployment
  "AV4S1dHHdvdt9GZpDzfwjhfgpY79J3w3kPrTWNvFQEuj"
);
const cdtToken = new web3.PublicKey( // CDT token mint address
  "AV4S1dHHdvdt9GZpDzfwjhfgpY79J3w3kPrTWNvFQEuj"
);

const wallet = new anchor.Wallet(
  web3.Keypair.fromSecretKey(Uint8Array.from(PrivateKey))
);

const connection = new web3.Connection(web3.clusterApiUrl("devnet"));
const provider = new anchor.AnchorProvider(connection, wallet, {});
const program = new anchor.Program(idl as anchor.Idl, programId, provider);

const addTransfersFrom = async () => {
  const [bridgeInfo] = await web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("bridge_info")],
    new web3.PublicKey(programId)
  );
  const [vaultsToken] = await anchor.web3.PublicKey.findProgramAddress(
    [
      anchor.utils.bytes.utf8.encode("bridge_token_vaults"),
      cdtToken.toBuffer(),
    ],
    program.programId
  );
  const walletATA = await splToken.getAssociatedTokenAddress(
    cdtToken,
    wallet.publicKey
  );

  // account to get transfers
  const newAccount1 = web3.Keypair.generate();
  await program.provider.connection.confirmTransaction(
    await program.provider.connection.requestAirdrop(
      newAccount1.publicKey,
      100 * web3.LAMPORTS_PER_SOL
    )
  );
  const newAccount2 = web3.Keypair.generate();
  await program.provider.connection.confirmTransaction(
    await program.provider.connection.requestAirdrop(
      newAccount2.publicKey,
      100 * web3.LAMPORTS_PER_SOL
    )
  );

  const newATA1 = await splToken.getAssociatedTokenAddress(
    cdtToken,
    newAccount1.publicKey
  );
  const newATA2 = await splToken.getAssociatedTokenAddress(
    cdtToken,
    newAccount2.publicKey
  );


  const tx = await program.methods
    .addTransfersFrom(
      ["1", "2"],
      [new anchor.BN(500), new anchor.BN(5000)],
      ["1", "2"]
    )
    .accounts({
      authority: wallet.publicKey,
      bridgeInfo,
      tokenProgram: splToken.TOKEN_PROGRAM_ID,
      associatedTokenProgram: splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenVaults: vaultsToken,
      systemProgram: web3.SystemProgram.programId,
      tokenMint: cdtToken,
      senderToken: walletATA,
    })
    .remainingAccounts([
      { pubkey: newAccount1.publicKey, isSigner: false, isWritable: true },
      { pubkey: newATA1, isSigner: false, isWritable: true },
      { pubkey: newAccount2.publicKey, isSigner: false, isWritable: true },
      { pubkey: newATA2, isSigner: false, isWritable: true },
    ])
    .signers([wallet.payer])
    .rpc();

  console.log(tx);
};
