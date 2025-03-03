use std::{env, path::Path};

use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Keypair,
    signer::{EncodableKey, Signer},
    transaction::Transaction,
};
use solana_transaction_status_client_types::{
    option_serializer::OptionSerializer, UiTransactionEncoding,
};
use xtasks::ops::cmd;

use clap::{arg, Command};

use color_eyre::eyre::{eyre, Ok, Result};

use tree_lib::{TreeInstruction, TREE_PROGRAM_SEED};

fn commands() -> Command {
    Command::new("Merkle xtasks")
        .arg(arg!(-d --deploy "Build and deploy the smart contract"))
        .arg(arg!(--info "Get merkle tree information"))
        .arg(arg!(--insert <HEX_VALUE> "Insert hex value into the tree"))
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let project_root = Path::new(&manifest_dir)
        .parent()
        .expect("Failed to find project root");

    println!("Project root: {:?}", project_root);
    let mut com = commands();
    let args_match = com.clone().get_matches();

    if !args_match.args_present() {
        com.print_help()?;
        return Ok(());
    }

    if args_match.get_flag("deploy") {
        build_deploy(project_root)?;
    }

    if args_match.get_flag("info") {
        get_tree_info(project_root)?;
    }

    if let Some(value) = args_match.get_one::<String>("insert") {
        let value = hex::decode(value)?;
        insert_leaf(project_root, value)?;
    }

    Ok(())
}

fn build_deploy(root: &Path) -> Result<()> {
    println!("Build/Deploy smart contract");

    let _ = cmd!("cargo", "build-sbf").dir(root).read().unwrap();

    let _output = cmd!(
        "solana",
        "program",
        "deploy",
        "--program-id",
        "./tree_program/keypair.json",
        "target/deploy/tree_program.so"
    )
    .dir(root)
    .read()?;

    // println!("Result {}", output);

    Ok(())
}

fn get_tree_info(root: &Path) -> Result<()> {
    let keypair_path = root.join("tree_program/keypair.json");
    let program_kp = Keypair::read_from_file(&keypair_path).map_err(|e| eyre!("Got {:?}", e))?;
    let program_id = program_kp.try_pubkey()?;

    // Connect to the Solana devnet
    let rpc_url = String::from("http://localhost:8899");
    let client = RpcClient::new(rpc_url);

    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let config_file = home_dir.join(".config/solana/id.json");

    let payer = Keypair::read_from_file(&config_file).map_err(|e| eyre!("Got {:?}", e))?;

    let (pda, _bump) = Pubkey::find_program_address(
        &[TREE_PROGRAM_SEED, &payer.try_pubkey()?.to_bytes()],
        &program_id,
    );

    let mut tx = create_transaction(&program_id, &payer, &pda, TreeInstruction::GetInfo);
    let recent_blockhash = client.get_latest_blockhash()?;
    tx.sign(&[payer], recent_blockhash);

    println!("Getting info");

    let response = client.simulate_transaction(&tx)?;

    if let Some(err) = response.value.err {
        println!("Error: {:?}", err);
    }

    if let Some(logs) = response.value.logs {
        println!("LOGS:");
        for log in logs {
            println!("-> {}", log);
        }
    }

    Ok(())
}

fn insert_leaf(root: &Path, value: Vec<u8>) -> Result<()> {
    let keypair_path = root.join("tree_program/keypair.json");
    let program_kp = Keypair::read_from_file(&keypair_path).map_err(|e| eyre!("Got {:?}", e))?;
    let program_id = program_kp.try_pubkey()?;

    // Connect to the Solana devnet
    let rpc_url = String::from("http://localhost:8899");
    let client = RpcClient::new(rpc_url);

    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let config_file = home_dir.join(".config/solana/id.json");

    let payer = Keypair::read_from_file(&config_file).map_err(|e| eyre!("Got {:?}", e))?;

    let (pda, _bump) = Pubkey::find_program_address(
        &[TREE_PROGRAM_SEED, &payer.try_pubkey()?.to_bytes()],
        &program_id,
    );

    println!("Inserting value {value:?}");

    let mut tx = create_transaction(&program_id, &payer, &pda, TreeInstruction::Insert { value });
    let recent_blockhash = client.get_latest_blockhash()?;
    tx.sign(&[payer], recent_blockhash);

    let tx_signature = client.send_and_confirm_transaction(&tx)?;

    let tx = client.get_transaction(&tx_signature, UiTransactionEncoding::Json)?;

    let tx_meta = tx.transaction.meta.expect("Transaction meta");

    if let Some(err) = tx_meta.err {
        println!("Error: {:?}", err);
    }

    if let OptionSerializer::Some(logs) = tx_meta.log_messages {
        println!("LOGS:");
        for log in logs {
            println!("-> {}", log);
        }
    }

    Ok(())
}

fn create_transaction(
    program_id: &Pubkey,
    payer: &Keypair,
    pda: &Pubkey,
    data: TreeInstruction,
) -> Transaction {
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(*pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: borsh::to_vec(&data).unwrap(),
    };

    Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()))
}
