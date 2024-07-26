import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import * as web3 from "@solana/web3.js";
import * as splToken from "@solana/spl-token";
import { SolanaCdtBridge } from "../target/types/solana_cdt_bridge";

let tokenMint: web3.PublicKey;
let bridgeInfo: web3.PublicKey;
let vaultsToken: web3.PublicKey;
let vaultsNative: web3.PublicKey;
let dexPool: web3.Keypair;
let dexPoolATA: web3.PublicKey;
let walletATA: web3.PublicKey;
let owner: web3.Keypair;

describe("solana-cdt-bridge", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.SolanaCdtBridge as Program<SolanaCdtBridge>;

  // before(async () => {
  //   console.log(program.programId.toBase58())
  //   tokenMint = new web3.PublicKey(
  //     "4v1BiaG2PhPvvxCrvYQdFXavRojMGw7FQ885uVQhbRnz"
  //   )
  //   walletATA = await splToken.getAssociatedTokenAddress(
  //     tokenMint,
  //     owner.publicKey
  //   )
  //   console.log(
  //     await program.provider.connection.getTokenAccountBalance(walletATA)
  //   )
  //   ;[bridgeInfo] = await anchor.web3.PublicKey.findProgramAddress(
  //     [anchor.utils.bytes.utf8.encode("bridge_info")],
  //     program.programId
  //   )
  //   ;[vaultsToken] = await anchor.web3.PublicKey.findProgramAddress(
  //     [anchor.utils.bytes.utf8.encode("bridge_token_vaults"), tokenMint.toBuffer()],
  //     program.programId
  //   )
  //   console.log(vaultsToken.toBase58())
  //   dexPool = pg.wallets.wallet1.publicKey
  //   dexPoolATA = await splToken.getAssociatedTokenAddress(tokenMint, dexPool)
  //   console.log(dexPool.toBase58(), dexPoolATA.toBase58())
  // })
  before(async () => {
    console.log(program.provider.connection.rpcEndpoint);
    owner = web3.Keypair.generate();

    await program.provider.connection.confirmTransaction(
      await program.provider.connection.requestAirdrop(
        owner.publicKey,
        100 * web3.LAMPORTS_PER_SOL
      )
    );

    console.log(
      "Owner: ",
      owner.publicKey.toBase58(),
      await program.provider.connection.getBalance(owner.publicKey)
    );

    tokenMint = await splToken.createMint(
      program.provider.connection,
      owner,
      owner.publicKey,
      null,
      6
    );
    console.log("token Mint:", tokenMint.toBase58());
    walletATA = await splToken.createAssociatedTokenAccount(
      program.provider.connection,
      owner,
      tokenMint,
      owner.publicKey
    );
    console.log("wallet ATA:", walletATA.toBase58());
    await splToken.mintTo(
      program.provider.connection,
      owner,
      tokenMint,
      walletATA,
      owner,
      1_000_000_000_000_000
    );
    console.log(
      (await program.provider.connection.getTokenAccountBalance(walletATA)).value.amount
    );
    [bridgeInfo] = await anchor.web3.PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("bridge_info")],
      program.programId
    );
    [vaultsToken] = await anchor.web3.PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("bridge_token_vaults"), tokenMint.toBuffer()],
      program.programId
    );
    [vaultsNative] = await anchor.web3.PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("bridge_native_vaults")],
      program.programId
    );

    dexPool = web3.Keypair.generate();

    await program.provider.connection.confirmTransaction(
      await program.provider.connection.requestAirdrop(
        dexPool.publicKey,
        100 * web3.LAMPORTS_PER_SOL
      )
    );

    console.log(
      "DexPool: ",
      dexPool.publicKey.toBase58(),
      await program.provider.connection.getBalance(dexPool.publicKey)
    );

    dexPoolATA = await splToken.createAssociatedTokenAccount(
      program.provider.connection,
      owner,
      tokenMint,
      dexPool.publicKey
    );
    await splToken.transfer(
      program.provider.connection,
      owner,
      walletATA,
      dexPoolATA,
      owner,
      1_000_000_000
    );
    console.log(dexPool.publicKey.toBase58(), dexPoolATA.toBase58());
  });
  it("initialize", async () => {
    // Generate keypair for the new account
    console.log(
      tokenMint.toBase58(),
      bridgeInfo.toBase58(),
      owner.publicKey.toBase58(),
      web3.SystemProgram.programId.toBase58()
    );
    // Send transaction
    const txHash = await program.methods
      .initialize("test", new anchor.BN(1_000_000_000_000), new anchor.BN(1))
      .accounts({
        bridgeInfo: bridgeInfo,
        authority: owner.publicKey,
        systemProgram: web3.SystemProgram.programId,
        tokenMint: tokenMint,
        tokenVaults: vaultsToken,
        nativeVaults: vaultsNative,
        dexInTokenMint: tokenMint,
        dexOutTokenMint: tokenMint,
        dexPool: dexPool.publicKey,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
      })
      .signers([owner])
      .rpc();
  });
  it("deposit", async () => {
    console.log(
      "before deposit: ",
      (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
    );
    const txHash = await program.methods
      .deposit(new anchor.BN(1000))
      .accounts({
        authority: owner.publicKey,
        bridgeInfo,
        senderToken: walletATA,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        tokenVaults: vaultsToken,
      })
      .signers([owner])
      .rpc();

    console.log(
      "after deposit: ",
      (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
    );
  });
  it("deposit & withdraw sol", async () => {
    console.log(
      "before deposit: ",
      await program.provider.connection.getBalance(vaultsNative)
    );
    const txHash1 = await program.methods
      .depositSol(new anchor.BN(20000))
      .accounts({
        authority: owner.publicKey,
        bridgeInfo,
        nativeVaults: vaultsNative,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    console.log(
      "after deposit: ",
      await program.provider.connection.getBalance(vaultsNative)
    );
    const txHash2 = await program.methods
      .withdrawSol(new anchor.BN(1000))
      .accounts({
        authority: owner.publicKey,
        bridgeInfo,
        nativeVaults: vaultsNative,
        systemProgram: web3.SystemProgram.programId,
        receiver: owner.publicKey
      })
      .signers([owner])
      .rpc();

    console.log(
      "after withdraw: ",
      await program.provider.connection.getBalance(vaultsNative)
    );
  });
  // it("set fees in dollar", async () => {
  //   // Send transaction
  //   const txHash = await program.methods
  //     .setFeesInDollar(new BN(2))
  //     .accounts({
  //       bridgeInfo: bridgeInfo,
  //       authority: owner.publicKey,
  //     })
  //     .signers([owner])
  //     .rpc();
  // });
  // it("set fees in cdt percentage", async () => {
  //   // Send transaction
  //   const txHash = await program.methods
  //     .setFeesInCdtPercentage(new BN(20))
  //     .accounts({
  //       bridgeInfo: bridgeInfo,
  //       authority: owner.publicKey,
  //     })
  //     .signers([owner])
  //     .rpc();
  // });
  // it("set minimum transfer quantity", async () => {
  //   // Send transaction
  //   const txHash = await program.methods
  //     .setMinimumTransferQuantity(new BN(1000000))
  //     .accounts({
  //       bridgeInfo: bridgeInfo,
  //       authority: owner.publicKey,
  //     })
  //     .signers([owner])
  //     .rpc();
  // });
  // it("update transfer cost", async () => {
  //   const txHash = await program.methods
  //     .updateTransferCost(new BN(100))
  //     .accounts({ bridgeInfo, authority: owner.publicKey })
  //     .signers([owner])
  //     .rpc();
  // });
  // it("change owner", async () => {
  //   // Send transaction
  //   const txHash = await program.methods
  //     .changeOwner()
  //     .accounts({
  //       bridgeInfo: bridgeInfo,
  //       owner: pg.wallets.wallet1.publicKey,
  //       authority: owner.publicKey,
  //     })
  //     .signers([owner])
  //     .rpc();
  // });
  // it("change program", async () => {
  //   // Send transaction
  //   const txHash = await program.methods
  //     .changeProgram()
  //     .accounts({
  //       bridgeInfo: bridgeInfo,
  //       program: pg.wallets.wallet1.publicKey,
  //       authority: owner.publicKey,
  //     })
  //     .signers([owner])
  //     .rpc();
  // });
  // it("ask withdraw", async () => {
  //   const txHash = await program.methods
  //     .askWithdraw()
  //     .accounts({ bridgeInfo, authority: owner.publicKey })
  //     .signers([owner])
  //     .rpc();
  // });
  // it("set paused", async () => {
  //   const txHash = await program.methods
  //     .setPaused(false)
  //     .accounts({ bridgeInfo, authority: owner.publicKey })
  //     .signers([owner])
  //     .rpc();
  // });
  it("init transfer", async () => {
    console.log(
      "before init: ",
      (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
    );
    for (let index = 0; index < 3; index++) {
      const [bridgeTransfer] = await anchor.web3.PublicKey.findProgramAddress(
        [
          anchor.utils.bytes.utf8.encode("bridge_transfer"),
          new anchor.BN(index).toArrayLike(Buffer, "le", 8),
        ],
        program.programId
      );
      const txHash = await program.methods
        .initTransfer(new anchor.BN("5000"), "to", "test")
        .accounts({
          authority: owner.publicKey,
          bridgeInfo,
          poolInToken: dexPoolATA,
          poolOutToken: dexPoolATA,
          senderToken: walletATA,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          tokenVaults: vaultsToken,
          bridgeTransfer,
        })
        .signers([owner])
        .rpc();
    }
    console.log(
      "after init: ",
      (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
    );
  });
  it("add transfers from", async () => {
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
      tokenMint,
      newAccount1.publicKey
    );
    const newATA2 = await splToken.getAssociatedTokenAddress(
      tokenMint,
      newAccount2.publicKey
    );

    const txHash = await program.methods
      .addTransfersFrom(
        ["1", "2"],
        [new anchor.BN(500), new anchor.BN(5000)],
        ["1", "2"]
      )
      .accounts({
        authority: owner.publicKey,
        bridgeInfo,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        associatedTokenProgram: splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenVaults: vaultsToken,
        systemProgram: web3.SystemProgram.programId,
        tokenMint: tokenMint,
        senderToken: walletATA,
      })
      .remainingAccounts([
        { pubkey: newAccount1.publicKey, isSigner: false, isWritable: true },
        { pubkey: newATA1, isSigner: false, isWritable: true },
        { pubkey: newAccount2.publicKey, isSigner: false, isWritable: true },
        { pubkey: newATA2, isSigner: false, isWritable: true },
      ])
      .signers([owner])
      .rpc();
    console.log('after add: ',
      (await program.provider.connection.getTokenAccountBalance(newATA1)).value.amount,
      (await program.provider.connection.getTokenAccountBalance(newATA2)).value.amount,
    );
  });
  it("collect fee cdts", async () => {
    console.log(
      "before collect: ",
      (await program.provider.connection.getTokenAccountBalance(walletATA)).value.amount,
      (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
    );
    const txHash = await program.methods
      .collectCdtFees()
      .accounts({
        authority: owner.publicKey,
        bridgeInfo,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        tokenVaults: vaultsToken,
        receiverToken: walletATA
      })
      .signers([owner])
      .rpc();

    console.log(
      "after collect: ",
      (await program.provider.connection.getTokenAccountBalance(walletATA)).value.amount,
      (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
    );
  });
  // it("withdraw", async () => {
  //   console.log(
  //     "before withdraw: ",
  //     (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
  //   );
  //   const txHash = await program.methods
  //     .withdraw(new anchor.BN(2000))
  //     .accounts({
  //       authority: owner.publicKey,
  //       bridgeInfo,
  //       tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //       tokenVaults: vaultsToken,
  //       receiverToken: walletATA
  //     })
  //     .signers([owner])
  //     .rpc();

  //   console.log(
  //     "after withdraw: ",
  //     (await program.provider.connection.getTokenAccountBalance(vaultsToken)).value.amount
  //   );
  // });
});
