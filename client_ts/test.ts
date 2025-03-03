import { getKeypairFromFile } from "@solana-developers/helpers";

const keyPair = await getKeypairFromFile();

console.log("Keypair", keyPair.publicKey.toBase58());