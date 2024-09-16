import { assert } from "chai";
import crypto from "crypto";
import BN from "bn.js";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { AnchorError, Program } from "@coral-xyz/anchor";
import { SolanaPastelOracle } from "../target/types/solana_pastel_oracle";

// PDAs

const REWARD_POOL_PREFIX = "reward_pool";
function findRewardPoolAddress(
  oracleAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(REWARD_POOL_PREFIX), oracleAddress.toBuffer()],
    programId
  )[0];
}

// Constants

// Account keypairs

export const ORACLE_KEYPAIR = Keypair.generate();

// Authority keypairs

const ADMIN_KEYPAIR = Keypair.generate();

describe("solana_pastel_oracle", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .SolanaPastelOracle as Program<SolanaPastelOracle>;

  const rewardPoolAddress = findRewardPoolAddress(
    ORACLE_KEYPAIR.publicKey,
    program.programId
  );

  it("Initialize an oracle", async () => {
    await program.methods
      .initialize()
      .accountsStrict({
        admin: ADMIN_KEYPAIR.publicKey,
        oracle: ORACLE_KEYPAIR.publicKey,
        rewardPool: rewardPoolAddress,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([
        await program.account.oracle.createInstruction(ORACLE_KEYPAIR),
      ])
      .signers([ORACLE_KEYPAIR, ADMIN_KEYPAIR])
      .rpc();
  });
});
