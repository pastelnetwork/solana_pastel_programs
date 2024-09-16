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
import { SolanaPastelBridge } from "../target/types/solana_pastel_bridge";
import { ORACLE_KEYPAIR } from "./solana_pastel_oracle";

// PDAs

const REWARD_POOL_PREFIX = "bridge_reward_pool";
function findBridgeRewardPoolAddress(
  bridgeAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(REWARD_POOL_PREFIX), bridgeAddress.toBuffer()],
    programId
  )[0];
}

const ESCROW_PREFIX = "bridge_escrow";
function findBridgeEscrowAddress(
  bridgeAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(ESCROW_PREFIX), bridgeAddress.toBuffer()],
    programId
  )[0];
}

const BRIDGE_NODE_PREFIX = "bridge_node";
function findBridgeNodeAddress(
  pastelId: Uint8Array,
  bridgeAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BRIDGE_NODE_PREFIX), pastelId, bridgeAddress.toBuffer()],
    programId
  )[0];
}

const SERVICE_REQUEST_PREFIX = "service_request";
function findServiceRequestAddress(
  pastelTicketTypeId: number, // u8
  first6CharsOfHash: string,
  userAddress: PublicKey,
  bridgeAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(SERVICE_REQUEST_PREFIX),
      Buffer.from([pastelTicketTypeId]),
      Buffer.from(first6CharsOfHash),
      userAddress.toBuffer(),
      bridgeAddress.toBuffer(),
    ],
    programId
  )[0];
}

const BEST_PRICE_QUOTE = "best_price_quote";
function findBestPriceQuoteAddress(
  serviceRequestAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BEST_PRICE_QUOTE), serviceRequestAddress.toBuffer()],
    programId
  )[0];
}

const SERVICE_PRICE_QUOTE = "service_price_quote";
function findServicePriceQuoteAddress(
  serviceRequestAddress: PublicKey,
  bridgeNodeAddress: PublicKey,
  programId: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(SERVICE_PRICE_QUOTE),
      serviceRequestAddress.toBuffer(),
      bridgeNodeAddress.toBuffer(),
    ],
    programId
  )[0];
}

// Constants

const BRIDGE_NODE_REGISTRATION_FEE_IN_LAMPORTS = 1_000_000; // Registration fee for bridge nodes in lamports
const NUM_BRIDGE_NODES = 3;
const NUMBER_OF_SIMULATED_SERVICE_REQUESTS = 5;

const baselinePriceUSD = 3; // Maximum price in USD
const solToUsdRate = 130; // 1 SOL = $130
const baselinePriceSol = baselinePriceUSD / solToUsdRate; // Convert USD to SOL

enum PastelTicketType {
  Sense = 0,
  Cascade = 1,
  Nft = 2,
  InferenceApi = 3,
}

function generateRandomPriceQuote(baselinePriceLamports: number): BN {
  const randomIncrement =
    Math.floor(Math.random() * (baselinePriceLamports / 10)) -
    baselinePriceLamports / 20;
  return new BN(baselinePriceLamports + randomIncrement);
}

// Account keypairs

const BRIDGE_KEYPAIR = Keypair.generate();

// Authority keypairs

const ADMIN_KEYPAIR = Keypair.generate();
const BRIDGE_NODE_USER_KEYPAIR = Keypair.generate();

describe("solana_pastel_bridge", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .SolanaPastelBridge as Program<SolanaPastelBridge>;

  const bridgeRewardPoolAddress = findBridgeRewardPoolAddress(
    BRIDGE_KEYPAIR.publicKey,
    program.programId
  );
  const bridgeEscrowAddress = findBridgeEscrowAddress(
    BRIDGE_KEYPAIR.publicKey,
    program.programId
  );

  it("Initialize a bridge", async () => {
    await program.methods
      .initialize()
      .accountsStrict({
        admin: ADMIN_KEYPAIR.publicKey,
        bridge: BRIDGE_KEYPAIR.publicKey,
        bridgeRewardPool: bridgeRewardPoolAddress,
        bridgeEscrow: bridgeEscrowAddress,
        oracle: ORACLE_KEYPAIR.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([
        await program.account.bridge.createInstruction(BRIDGE_KEYPAIR),
      ])
      .signers([BRIDGE_KEYPAIR, ADMIN_KEYPAIR])
      .rpc();

    const bridge = await program.account.bridge.fetch(BRIDGE_KEYPAIR.publicKey);
    assert.deepEqual(bridge.admin, ADMIN_KEYPAIR.publicKey);
    assert.deepEqual(bridge.oracle, ORACLE_KEYPAIR.publicKey);
  });

  it("Register new bridge nodes", async () => {
    for (let i = 0; i < NUM_BRIDGE_NODES; i++) {
      const uniquePastelId = crypto.randomBytes(32);
      const pastelId = Array.from(uniquePastelId);
      console.log(
        `>>>> Generated random Pastel ID[${i}]: ${uniquePastelId.toString(
          "hex"
        )}`
      );

      const uniquePslAddress = "P" + crypto.randomBytes(33).toString("hex");
      console.log(`Generated random Pastel address: ${uniquePslAddress}`);

      const bridgeNodeAddress = findBridgeNodeAddress(
        uniquePastelId,
        BRIDGE_KEYPAIR.publicKey,
        program.programId
      );

      await program.methods
        .registerNewBridgeNode(pastelId, uniquePslAddress)
        .accountsStrict({
          user: BRIDGE_NODE_USER_KEYPAIR.publicKey,
          payer: provider.publicKey, // user can be payer by himself
          bridge: BRIDGE_KEYPAIR.publicKey,
          bridgeNode: bridgeNodeAddress,
          bridgeRewardPool: bridgeRewardPoolAddress,
          systemProgram: SystemProgram.programId,
        })
        .signers([BRIDGE_NODE_USER_KEYPAIR])
        .rpc();

      const bridgeNode = await program.account.bridgeNode.fetch(
        bridgeNodeAddress
      );
      assert.deepEqual(bridgeNode.bridge, BRIDGE_KEYPAIR.publicKey);
      assert.deepEqual(bridgeNode.pastelId, pastelId);
      assert.equal(bridgeNode.bridgeNodePslAddress, uniquePslAddress);
    }

    const bridgeRewardPoolBalance = await provider.connection.getBalance(
      bridgeRewardPoolAddress
    );
    assert.equal(
      bridgeRewardPoolBalance,
      BRIDGE_NODE_REGISTRATION_FEE_IN_LAMPORTS * NUM_BRIDGE_NODES
    );

    const bridge = await program.account.bridge.fetch(BRIDGE_KEYPAIR.publicKey);
    assert.equal(bridge.bridgeNodesCount, NUM_BRIDGE_NODES);
  });

  it("Submit service requests", async () => {
    for (let i = 0; i < NUMBER_OF_SIMULATED_SERVICE_REQUESTS; i++) {
      const fileHash = crypto
        .createHash("sha3-256")
        .update(`file${i}`)
        .digest("hex")
        .substring(0, 6);
      console.log(`>>>> Generated file hash for file[${i}]: ${fileHash}`);

      const pastelTicketId = i % 4;
      const pastelTicketType = PastelTicketType[pastelTicketId];
      let type = null;
      switch (pastelTicketType) {
        case "Sense":
          type = { sense: {} };
          break;
        case "Cascade":
          type = { cascade: {} };
          break;
        case "Nft":
          type = { nft: {} };
          break;
        case "InferenceApi":
          type = { inferenceApi: {} };
          break;
        default:
          throw Error("Invalid pastel ticket type");
      }
      console.log(`Selected PastelTicketType: ${pastelTicketType}`);

      const ipfsCid = `Qm${crypto.randomBytes(44).toString("hex")}`;
      console.log(`Generated IPFS CID: ${ipfsCid}`);

      const fileSizeBytes = new BN(Math.floor(Math.random() * 1000000) + 1);
      console.log(
        `Generated random file size: ${fileSizeBytes.toString()} bytes`
      );

      const REQUESTOR_KEYPAIR = Keypair.generate();

      const serviceRequestAddress = findServiceRequestAddress(
        pastelTicketId,
        fileHash,
        REQUESTOR_KEYPAIR.publicKey,
        BRIDGE_KEYPAIR.publicKey,
        program.programId
      );

      await program.methods
        .submitServiceRequest(type, fileHash, ipfsCid, fileSizeBytes)
        .accountsStrict({
          user: REQUESTOR_KEYPAIR.publicKey,
          payer: provider.publicKey, // requestor can be payer by himself
          bridge: BRIDGE_KEYPAIR.publicKey,
          serviceRequest: serviceRequestAddress,
          systemProgram: SystemProgram.programId,
        })
        .signers([REQUESTOR_KEYPAIR])
        .rpc();

      const request = await program.account.serviceRequest.fetch(
        serviceRequestAddress
      );
      assert.deepEqual(request.bridge, BRIDGE_KEYPAIR.publicKey);
      assert.deepEqual(request.user, REQUESTOR_KEYPAIR.publicKey);
      assert.deepEqual(request.serviceType, type);
      assert.equal(
        request.first6CharactersOfSha3256HashOfCorrespondingFile,
        fileHash
      );
      assert.equal(request.ipfsCid, ipfsCid);
      assert.equal(request.fileSizeBytes.toString(), fileSizeBytes.toString());
    }

    const bridge = await program.account.bridge.fetch(BRIDGE_KEYPAIR.publicKey);
    assert.equal(
      bridge.serviceRequestsCount,
      NUMBER_OF_SIMULATED_SERVICE_REQUESTS
    );
  });

  it("Submit price quotes for service requests", async () => {
    const requests = await program.account.serviceRequest.all();
    assert.equal(requests.length, NUMBER_OF_SIMULATED_SERVICE_REQUESTS);

    const bridgeNodes = await program.account.bridgeNode.all();
    assert.equal(bridgeNodes.length, NUM_BRIDGE_NODES);

    for (const request of requests) {
      const bestPriceQuoteAddress = findBestPriceQuoteAddress(
        request.publicKey,
        program.programId
      );

      await program.methods
        .initializeBestPriceQuote()
        .accountsStrict({
          payer: provider.publicKey, // permissionless
          serviceRequest: request.publicKey,
          bestPriceQuote: bestPriceQuoteAddress,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      for (const bridgeNode of bridgeNodes) {
        const servicePriceQuoteAddress = findServicePriceQuoteAddress(
          request.publicKey,
          bridgeNode.publicKey,
          program.programId
        );

        const baselinePriceLamports = Math.floor(
          (request.account.fileSizeBytes.toNumber() / 1000000) *
            baselinePriceSol *
            LAMPORTS_PER_SOL
        );
        const quotedPriceLamports = generateRandomPriceQuote(
          baselinePriceLamports
        );

        try {
          await program.methods
            .submitPriceQuote(quotedPriceLamports)
            .accounts({
              rewardAddress: BRIDGE_NODE_USER_KEYPAIR.publicKey,
              payer: provider.publicKey,
              bridge: BRIDGE_KEYPAIR.publicKey,
              bridgeNode: bridgeNode.publicKey,
              serviceRequest: request.publicKey,
              bestPriceQuote: bestPriceQuoteAddress,
              servicePriceQuote: servicePriceQuoteAddress,
              systemProgram: SystemProgram.programId,
            })
            .signers([BRIDGE_NODE_USER_KEYPAIR])
            .rpc();
        } catch (err) {
          if (err instanceof AnchorError) {
            console.error(`Error code: ${err.error.errorCode.number}`);
            console.error(`Error message: ${err.error.errorMessage}`);
          }
        }
      }

      const bestPriceQuote = await program.account.bestPriceQuote.fetch(
        bestPriceQuoteAddress
      );
      assert.deepEqual(bestPriceQuote.serviceRequest, request.publicKey);
      assert.isNotOk(bestPriceQuote.bestQuotedPriceInLamports.eqn(0));
    }
  });

  it("Submits Pastel TxIDs for service requests", async () => {
    const requests = await program.account.serviceRequest.all();
    assert.equal(requests.length, NUMBER_OF_SIMULATED_SERVICE_REQUESTS);

    for (const request of requests) {
      assert.isNotNull(request.account.selectedBridgeNode);

      const pastelTxid = crypto.randomBytes(32).toString("hex"); // Generate a random Pastel TxID

      await program.methods
        .submitPastelTxid(pastelTxid)
        .accountsStrict({
          user: request.account.user,
          serviceRequest: request.publicKey,
          bridge: request.account.bridge,
          bridgeNode: request.account.selectedBridgeNode,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }
  });
});
