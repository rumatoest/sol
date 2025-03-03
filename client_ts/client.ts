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

const keyPair: Keypair = await getKeypairFromFile();

console.log("Keypair", keyPair.publicKey.toBase58());

const connection = new Connection("http://localhost:8899", "confirmed");

const blockhashInfo = await connection.getLatestBlockhash();

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

const txOut = await connection.getTransaction(txHash);

console.log("Transaction details", txOut);
