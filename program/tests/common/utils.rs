use agnostic_orderbook::state::{EVENT_QUEUE_HEADER_LEN, MARKET_STATE_LEN, REGISTER_SIZE};
use dex_v4::CALLBACK_INFO_LEN;
use solana_program::instruction::Instruction;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction::create_account;
use solana_program_test::BanksClientError;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::account::Account;
use solana_sdk::signature::Signer;
use solana_sdk::{signature::Keypair, transaction::Transaction};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::state::Mint;
use std::str::FromStr;

pub async fn sign_send_instructions(
    ctx: &mut ProgramTestContext,
    instructions: Vec<Instruction>,
    signers: Vec<&Keypair>,
) -> Result<(), BanksClientError> {
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&ctx.payer.pubkey()));
    let mut payer_signers = vec![&ctx.payer];
    for s in signers {
        payer_signers.push(s);
    }
    transaction.partial_sign(&payer_signers, ctx.last_blockhash);
    ctx.banks_client.process_transaction(transaction).await
}

pub async fn create_associated_token(
    prg_test_ctx: &mut ProgramTestContext,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<Pubkey, BanksClientError> {
    let create_associated_instruction =
        create_associated_token_account(&prg_test_ctx.payer.pubkey(), owner, mint);
    let associated_key = get_associated_token_address(owner, mint);

    sign_send_instructions(prg_test_ctx, vec![create_associated_instruction], vec![])
        .await
        .map(|()| associated_key)
}

pub type MintInfo = (Pubkey, Mint);

pub fn mint_bootstrap(
    address: Option<&str>,
    decimals: u8,
    program_test: &mut ProgramTest,
    mint_authority: &Pubkey,
) -> MintInfo {
    let address = address
        .map(|s| Pubkey::from_str(s).unwrap())
        .unwrap_or_else(Pubkey::new_unique);
    let mint_info = Mint {
        mint_authority: Some(*mint_authority).into(),
        supply: u32::MAX.into(),
        decimals,
        is_initialized: true,
        freeze_authority: None.into(),
    };
    let mut data = [0; Mint::LEN];
    mint_info.pack_into_slice(&mut data);
    program_test.add_account(
        address,
        Account {
            lamports: u32::MAX.into(),
            data: data.into(),
            owner: spl_token::ID,
            executable: false,
            ..Account::default()
        },
    );
    (address, mint_info)
}

pub struct AOBAccounts {
    pub event_queue: Pubkey,
    pub market: Pubkey,
    pub asks: Pubkey,
    pub bids: Pubkey,
}

/// Creates the accounts needed for the AAOB market testing and returns the
/// address of the market.
pub async fn create_aob_market_and_accounts(
    prg_test_ctx: &mut ProgramTestContext,
    dex_program_id: Pubkey,
) -> AOBAccounts {
    let rent = prg_test_ctx.banks_client.get_rent().await.unwrap();

    // Create market state account
    let market_account = Keypair::new();
    let aob_market_rent = rent.minimum_balance(MARKET_STATE_LEN);
    let create_market_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &market_account.pubkey(),
        aob_market_rent,
        agnostic_orderbook::state::MARKET_STATE_LEN as u64,
        &dex_program_id,
    );
    sign_send_instructions(
        prg_test_ctx,
        vec![create_market_account_instruction],
        vec![&market_account],
    )
    .await
    .unwrap();

    // Create event queue account
    let event_queue_account = Keypair::new();
    let evq_space = agnostic_orderbook::state::EVENT_QUEUE_HEADER_LEN
        + REGISTER_SIZE
        + 10 * (agnostic_orderbook::state::Event::compute_slot_size(CALLBACK_INFO_LEN as usize));
    let create_event_queue_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &event_queue_account.pubkey(),
        rent.minimum_balance(evq_space),
        evq_space as u64,
        &dex_program_id,
    );
    sign_send_instructions(
        prg_test_ctx,
        vec![create_event_queue_account_instruction],
        vec![&event_queue_account],
    )
    .await
    .unwrap();

    // Create bids account
    let bids_account = Keypair::new();
    let create_bids_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &bids_account.pubkey(),
        rent.minimum_balance(1_000_000),
        1_000_000,
        &dex_program_id,
    );
    sign_send_instructions(
        prg_test_ctx,
        vec![create_bids_account_instruction],
        vec![&bids_account],
    )
    .await
    .unwrap();

    // Create asks account
    let asks_account = Keypair::new();
    let create_asks_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &asks_account.pubkey(),
        rent.minimum_balance(1_000_000),
        1_000_000,
        &dex_program_id,
    );
    sign_send_instructions(
        prg_test_ctx,
        vec![create_asks_account_instruction],
        vec![&asks_account],
    )
    .await
    .unwrap();

    AOBAccounts {
        event_queue: event_queue_account.pubkey(),
        market: market_account.pubkey(),
        asks: asks_account.pubkey(),
        bids: bids_account.pubkey(),
    }
}
