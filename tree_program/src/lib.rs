use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use tree_lib::{MerkleTree, TreeInstruction, TREE_PROGRAM_SEED};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = TreeInstruction::try_from_slice(input)?;

    let accounts_iter = &mut accounts.iter();

    let funding_account = next_account_info(accounts_iter)?;
    let pda_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let (pda, pda_bump) = Pubkey::find_program_address(
        &[TREE_PROGRAM_SEED, &funding_account.key.to_bytes()],
        program_id,
    );

    // Ensure provided PDA matches derived PDA
    if pda != *pda_account.key {
        msg!("Invalid PDA provided");
        return Err(ProgramError::InvalidArgument);
    }

    match instruction {
        TreeInstruction::Insert { value } => {
            if pda_account.data_is_empty() || pda_account.lamports() == 0 {
                insert_init(
                    program_id,
                    funding_account,
                    pda_account,
                    system_program,
                    pda_bump,
                    value,
                )?;
            } else {
                insert_update(funding_account, pda_account, system_program, value)?;
            }
        }
        TreeInstruction::GetInfo => {
            if pda_account.data_is_empty() || pda_account.lamports() == 0 {
                msg!("EMPTY: no data found");
            } else {
                get_tree_info(pda_account)?;
            }
        }
    }

    Ok(())
}

fn create_pda_space<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system: &AccountInfo<'a>,
    bump: u8,
    space: usize,
) -> ProgramResult {
    let rent_lamports = Rent::get()?.minimum_balance(space);

    let create_ix = system_instruction::create_account(
        payer.key,     // Funding account
        pda.key,       // New account (PDA)
        rent_lamports, // Lamports for rent exemption
        space as u64,  // Space required for data
        program_id,    // Owner of the account
    );

    let signers_seeds: &[&[u8]; 3] = &[TREE_PROGRAM_SEED, &payer.key.to_bytes(), &[bump]];

    invoke_signed(
        &create_ix,
        &[payer.clone(), pda.clone(), system.clone()],
        &[signers_seeds],
    )
}

fn update_pda_space<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system: &AccountInfo<'a>,
    new_space: usize,
) -> ProgramResult {
    if pda.data_len() >= new_space {
        return pda.realloc(new_space, true);
    }

    let rent = Rent::get()?;
    let new_minimum_balance = rent.minimum_balance(new_space);

    let lamports_diff = new_minimum_balance.saturating_sub(pda.lamports());

    invoke(
        &system_instruction::transfer(payer.key, pda.key, lamports_diff),
        &[payer.clone(), pda.clone(), system.clone()],
    )?;

    return pda.realloc(new_space, true);
}

fn insert_init<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system: &AccountInfo<'a>,
    bump: u8,
    value: Vec<u8>,
) -> ProgramResult {
    msg!("Init with value: {:?}", value);

    let tree = MerkleTree::new(&[value]);

    let tree_vec = borsh::to_vec(&tree)?;

    create_pda_space(program_id, payer, pda, system, bump, tree_vec.len())?;

    let mut data = pda.try_borrow_mut_data()?;
    data.copy_from_slice(&tree_vec);

    msg!(
        "New tree size {} root hash {}",
        tree.leaf_count(),
        tree.get_root().expect("Must always beeeee")
    );

    Ok(())
}

fn insert_update<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system: &AccountInfo<'a>,
    value: Vec<u8>,
) -> ProgramResult {
    msg!("Appending value: {:?}", value);

    let data = pda.try_borrow_data()?;
    let old_tree = borsh::from_slice::<MerkleTree>(&data)?;
    let tree = MerkleTree::from_tree(old_tree, &[value]);

    drop(data);

    let tree_vec = borsh::to_vec(&tree)?;

    update_pda_space(payer, pda, system, tree_vec.len())?;

    let mut data = pda.try_borrow_mut_data()?;
    data.copy_from_slice(&tree_vec);

    msg!(
        "Updated tree size {} root hash {}",
        tree.leaf_count(),
        tree.get_root().expect("Must always beeeee")
    );

    Ok(())
}

fn get_tree_info(pda: &AccountInfo) -> ProgramResult {
    let data = pda.try_borrow_data()?;

    let tree = borsh::from_slice::<MerkleTree>(&data)?;

    msg!(
        "Tree size {} root hash {}",
        tree.leaf_count(),
        tree.get_root().expect("Must alway bee")
    );

    Ok(())
}

#[cfg(test)]
mod test {

    use super::*;

    use borsh::to_vec;
    use solana_hash::Hash;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_tree() {
        let program_id = Pubkey::new_unique();
        let mut program_test = ProgramTest::default();
        program_test.add_program("tree", program_id, processor!(process_instruction));

        let (mut banks_client, payer, mut recent_blockhash) = program_test.start().await;

        let (pda, _bump) = Pubkey::find_program_address(
            &[TREE_PROGRAM_SEED, &payer.pubkey().to_bytes()],
            &program_id,
        );

        // #1 getting info
        let transaction = create_signed_transaction(
            &program_id,
            &payer,
            &pda,
            recent_blockhash,
            TreeInstruction::GetInfo,
        );

        let tx_result = banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap();

        // Should not fail
        assert!(matches!(tx_result.result, Ok(_)), "{:?}", tx_result);

        let pda_account = banks_client.get_account(pda).await.unwrap();

        // Account does not exists now
        assert!(matches!(pda_account, None));

        // #2 Inserting node, creating account
        let transaction = create_signed_transaction(
            &program_id,
            &payer,
            &pda,
            recent_blockhash,
            TreeInstruction::Insert {
                value: vec![1, 1, 1],
            },
        );

        let tx_result = banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap();

        assert!(matches!(tx_result.result, Ok(_)), "{:?}", tx_result);

        let pda_account = banks_client.get_account(pda).await.unwrap();
        assert!(matches!(pda_account, Some(_)));

        let pda_account = pda_account.unwrap();
        let data = pda_account.data.as_slice();

        let tree = borsh::from_slice::<MerkleTree>(data).unwrap();
        let tree_check = MerkleTree::new(&[&[1, 1, 1]]);
        assert_eq!(tree.get_root(), tree_check.get_root());

        // #3 Appending node to the existing account
        let transaction = create_signed_transaction(
            &program_id,
            &payer,
            &pda,
            recent_blockhash,
            TreeInstruction::Insert {
                value: vec![2, 2, 2],
            },
        );

        let tx_result = banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap();

        assert!(matches!(tx_result.result, Ok(_)), "{:?}", tx_result);

        let pda_account = banks_client.get_account(pda).await.unwrap();
        assert!(matches!(pda_account, Some(_)));

        let pda_account = pda_account.unwrap();
        let data = pda_account.data.as_slice();

        let tree = borsh::from_slice::<MerkleTree>(data).unwrap();
        let tree_check = MerkleTree::new(&[&[1, 1, 1], &[2, 2, 2]]);
        assert_eq!(tree.get_root(), tree_check.get_root());

        // #4 Getting info from existing account
        recent_blockhash = banks_client
            .get_new_latest_blockhash(&recent_blockhash)
            .await
            .unwrap();
        let transaction = create_signed_transaction(
            &program_id,
            &payer,
            &pda,
            recent_blockhash,
            TreeInstruction::GetInfo,
        );

        let tx_result = banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap();

        // Should not fail
        assert!(matches!(tx_result.result, Ok(_)), "{:?}", tx_result);
    }

    fn create_signed_transaction(
        program_id: &Pubkey,
        payer: &Keypair,
        pda: &Pubkey,
        recent_blockhash: Hash,
        data: TreeInstruction,
    ) -> Transaction {
        let instruction = Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(*pda, false),
                AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            ],
            data: to_vec(&data).unwrap(),
        };
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));

        // Sign transaction
        transaction.sign(&[payer], recent_blockhash);

        transaction
    }
}
