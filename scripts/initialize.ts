import * as anchor from "@coral-xyz/anchor";
import * as web3 from "@solana/web3.js";
import splToken from "@solana/spl-token";

import idl from "../target/idl/solana_cdt_bridge.json";
import PrivateKey from "./privateKey.json";

const programId = new web3.PublicKey( // Bridge program id from the deployment
  "AV4S1dHHdvdt9GZpDzfwjhfgpY79J3w3kPrTWNvFQEuj"
);
const dexPool = new web3.PublicKey( // USDC / WSOL pair address
  "AV4S1dHHdvdt9GZpDzfwjhfgpY79J3w3kPrTWNvFQEuj"
);
const cdtToken = new web3.PublicKey( // CDT token mint address
  "AV4S1dHHdvdt9GZpDzfwjhfgpY79J3w3kPrTWNvFQEuj"
);
const usdcToken = new web3.PublicKey( // USDC token mint address
  "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
);
const wsolToken = new web3.PublicKey( // WSOL mint address
  "So11111111111111111111111111111111111111112"
);

const wallet = new anchor.Wallet(
  web3.Keypair.fromSecretKey(Uint8Array.from(PrivateKey))
);

const connection = new web3.Connection(web3.clusterApiUrl("devnet"));
const provider = new anchor.AnchorProvider(connection, wallet, {});
const program = new anchor.Program(idl as anchor.Idl, programId, provider);

const initialize = async () => {
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
  const [vaultsNative] = await anchor.web3.PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode("bridge_native_vaults")],
    program.programId
  );
  const tx = await program.methods
    .initialize("test", new anchor.BN(1_000_000_000_000), new anchor.BN(1))
    .accounts({
      bridgeInfo,
      authority: wallet.publicKey,
      systemProgram: web3.SystemProgram.programId,
      tokenMint: cdtToken,
      tokenVaults: vaultsToken,
      nativeVaults: vaultsNative,
      dexInTokenMint: usdcToken,
      dexOutTokenMint: wsolToken,
      dexPool: dexPool,
      tokenProgram: splToken.TOKEN_PROGRAM_ID,
    })
    .signers([wallet.payer])
    .rpc();

  console.log(tx);
};
