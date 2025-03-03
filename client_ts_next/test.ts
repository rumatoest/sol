import { loadKeypairSignerFromFile } from "gill/node";

import { fromLegacyKeypair } from '@solana/compat';


import { KeyPairSigner } from "@solana/signers";
import { pipe } from "@solana/functional";


import { Address, address, createSolanaRpc, createTransactionMessage, setTransactionMessageFeePayer, setTransactionMessageLifetimeUsingBlockhash, signTransaction } from '@solana/kit';

const programId: Address = address("4in4H8KQgeyd1WDCkuoQbhev88SRWqhwhxSomihAgP19");

const keyPair: KeyPairSigner = await loadKeypairSignerFromFile();
const feePayerAddress = address(keyPair.address);

console.log("Keypair ADDRESS", keyPair.address);

const rpc = createSolanaRpc("http://localhost:8899");

// console.log("Connection", rpc);

const blockhashInfo = await rpc.getLatestBlockhash().send();

console.log("Blockhash", blockhashInfo);

const transactionMessage = pipe(
    createTransactionMessage({ version: 'legacy' }),
    tx => setTransactionMessageFeePayer(feePayerAddress, tx),
    tx => setTransactionMessageLifetimeUsingBlockhash(blockhashInfo.value, tx)
);

console.log("Transaction Message", transactionMessage);


const signedTransaction = await signTransaction([keyPair.keyPair], transactionMessage);

console.log("Signed transaction", signedTransaction);




// const signedTransaction = await signTransaction([keyPair.keyPair], transactionMessageWithFeePayer);
