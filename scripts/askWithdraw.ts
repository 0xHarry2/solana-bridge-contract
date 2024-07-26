import * as anchor from "@coral-xyz/anchor";
import * as web3 from "@solana/web3.js";

import idl from "../target/idl/solana_cdt_bridge.json";
import PrivateKey from "./privateKey.json";

const programId = new web3.PublicKey( // Bridge program id from the deployment
  "AV4S1dHHdvdt9GZpDzfwjhfgpY79J3w3kPrTWNvFQEuj"
);

const wallet = new anchor.Wallet(
  web3.Keypair.fromSecretKey(Uint8Array.from(PrivateKey))
);

const connection = new web3.Connection(web3.clusterApiUrl("devnet"));
const provider = new anchor.AnchorProvider(connection, wallet, {});
const program = new anchor.Program(idl as anchor.Idl, programId, provider);

const askWithdraw = async () => {
  const [bridgeInfo] = await web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("bridge_info")],
    new web3.PublicKey(programId)
  );
  const tx = await program.methods
    .askWithdraw()
    .accounts({ bridgeInfo, authority: wallet.publicKey })
    .signers([wallet.payer])
    .rpc();

  console.log(tx);
};
