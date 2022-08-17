import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Vesting } from "../target/types/vesting";
import assert from "assert";



describe("pq_staking", () => {
    // Configure the client to use the local cluster.
    anchor.setProvider(anchor.Provider.env());

    const program = anchor.workspace.PqStaking as Program<Vesting>;

      it('can create vault and store vesting Information!', async () => {
          try{
              const [stakeVaultPda, stakeVaultBump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from('stakeVault')], program.programId);
              console.log("PDA: ", stakeVaultPda.toBase58());  // PDA: 94vMQdL1XavExQYii3V5P2yJjFD54a2jayB8HKadbEoL   //CLMr3m5RjScqxmRFwAnn6SWdBjTj3CY132EGcSPMvSMc //BcX2VZ1grFkqvL6uh2sX5cySPHWivuv7H5dnZyAjWB87 PDA for me
              let length = new anchor.BN(10e9);

              let admin= program.provider.wallet.publicKey;

              const txn = await program.rpc.createStakeVault(stakeVaultBump,  length, {
                  accounts: {
                      signer: program.provider.wallet.publicKey,
                      vaultAccount: stakeVaultPda,
                      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                      systemProgram: anchor.web3.SystemProgram.programId,
                  }
              });
              assert.ok(true);
              console.log("vault PDA  transaction signature: ", txn);
          } catch (e) {
              assert.equal(e.message, "failed to send transaction: Transaction simulation failed: Error processing Instruction 0: custom program error: 0x0");
          }

      });

     const stakeAccount  = anchor.web3.Keypair.generate();
       it('can send sols and store user state!', async () => {
         let amount = new anchor.BN(2e6);
         const [stakeVaultPda, _stakeVaultBump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from('stakeVault')], program.programId);
          const stake_tx =  await program.rpc.stakeToken(amount, {
              accounts: {
                  stakeAccount: stakeAccount.publicKey,
                  sender: program.provider.wallet.publicKey,
                  systemProgram: anchor.web3.SystemProgram.programId,
                  vault: stakeVaultPda,

              },
              signers:[stakeAccount]
          });

          console.log("can send sols: ", stake_tx);

      });

  });