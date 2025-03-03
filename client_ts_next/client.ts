import { getKeypairFromFile } from "@solana-developers/helpers";
import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  Keypair,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

const programId = new PublicKey("4in4H8KQgeyd1WDCkuoQbhev88SRWqhwhxSomihAgP19");

console.log("Program ID", programId.toBase58());

// Connect to a solana cluster. Either to your local test validator or to devnet
const connection = new Connection("http://localhost:8899", "confirmed");
//const connection = new Connection("https://api.devnet.solana.com", "confirmed");

async function main() {
  // We load the keypair that we created in a previous step
  const keyPair: Keypair = await getKeypairFromFile("~/.config/solana/id.json");

  console.log("Keypair", keyPair.publicKey.toBase58());

  // Every transaction requires a blockhash
  const blockhashInfo = await connection.getLatestBlockhash();

  console.log("Blockhash", blockhashInfo.blockhash);

  // Create a new transaction
  const tx = new Transaction({
    ...blockhashInfo,
  });

  // Add our Hello World instruction
  tx.add(
    new TransactionInstruction({
      programId: programId,
      keys: [],
      data: Buffer.from([]),
    }),
  );

  // Sign the transaction with your previously created keypair
  tx.sign(keyPair);

  // Send the transaction to the Solana network
  const txHash = await connection.sendRawTransaction(tx.serialize());

  console.log("Transaction sent with hash:", txHash);

  await connection.confirmTransaction({
    blockhash: blockhashInfo.blockhash,
    lastValidBlockHeight: blockhashInfo.lastValidBlockHeight,
    signature: txHash,
  });

  console.log(
    `Congratulations! Look at your ‘Hello World' transaction in the Solana Explorer:
    https://explorer.solana.com/tx/${txHash}?cluster=custom`,
  );
}

main().catch((err) => {
  console.error(err);
});