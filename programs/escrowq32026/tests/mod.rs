use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction, program_pack::Pack,
            system_program::ID as SYSTEM_PROGRAM_ID,
        },
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{associated_token, token::spl_token},
    litesvm::LiteSVM,
    litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo, spl_token::ID as TOKEN_PROGRAM_ID},
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

const SEED: u64 = 123;
const DEPOSIT: u64 = 10_000_000;
const RECEIVE: u64 = 7_000_000;
const EXPIRATION: i64 = 2_000_000_000;

struct Fixture {
    svm: LiteSVM,
    maker: Keypair,
    taker: Keypair,
    mint_a: Pubkey,
    mint_b: Pubkey,
    maker_ata_a: Pubkey,
    taker_ata_b: Pubkey,
    escrow: Pubkey,
    vault: Pubkey,
}

fn setup() -> Fixture {
    let mut svm = LiteSVM::new();
    svm.add_program(
        escrowq32026::id(),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/escrowq32026.so")),
    )
    .unwrap();
    let maker = Keypair::new();
    let taker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 5_000_000_000).unwrap();
    svm.airdrop(&taker.pubkey(), 5_000_000_000).unwrap();
    let mint_a = CreateMint::new(&mut svm, &maker).decimals(6).authority(&maker.pubkey()).send().unwrap();
    let mint_b = CreateMint::new(&mut svm, &maker).decimals(6).authority(&maker.pubkey()).send().unwrap();
    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&maker.pubkey()).send().unwrap();
    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
        .owner(&taker.pubkey()).send().unwrap();
    MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 100_000_000).send().unwrap();
    MintTo::new(&mut svm, &maker, &mint_b, &taker_ata_b, 100_000_000).send().unwrap();
    let escrow = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &SEED.to_le_bytes()],
        &escrowq32026::id(),
    ).0;
    let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
    Fixture { svm, maker, taker, mint_a, mint_b, maker_ata_a, taker_ata_b, escrow, vault }
}

fn send(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) -> bool {
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], msg, svm.latest_blockhash());
    svm.send_transaction(tx).is_ok()
}

fn make(f: &mut Fixture, deposit: u64, receive: u64, expiration: i64) -> bool {
    let ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Make {
            maker: f.maker.pubkey(), mint_a: f.mint_a, mint_b: f.mint_b,
            maker_ata_a: f.maker_ata_a, escrow: f.escrow, vault: f.vault,
            associated_token_program: associated_token::ID,
            token_program: TOKEN_PROGRAM_ID, system_program: SYSTEM_PROGRAM_ID,
        }.to_account_metas(None),
        data: escrowq32026::instruction::Make { seed: SEED, deposit, receive, expiration }.data(),
    };
    send(&mut f.svm, &f.maker, ix)
}

fn take(f: &mut Fixture) -> bool {
    let maker_ata_b = associated_token::get_associated_token_address(&f.maker.pubkey(), &f.mint_b);
    let taker_ata_a = associated_token::get_associated_token_address(&f.taker.pubkey(), &f.mint_a);
    let ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Take {
            taker: f.taker.pubkey(), maker: f.maker.pubkey(),
            mint_a: f.mint_a, mint_b: f.mint_b,
            taker_ata_b: f.taker_ata_b, maker_ata_b, taker_ata_a,
            escrow: f.escrow, vault: f.vault,
            associated_token_program: associated_token::ID,
            token_program: TOKEN_PROGRAM_ID, system_program: SYSTEM_PROGRAM_ID,
        }.to_account_metas(None),
        data: escrowq32026::instruction::Take {}.data(),
    };
    send(&mut f.svm, &f.taker, ix)
}

fn refund(f: &mut Fixture) -> bool {
    let ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Refund {
            maker: f.maker.pubkey(), mint_a: f.mint_a,
            maker_ata_a: f.maker_ata_a, escrow: f.escrow, vault: f.vault,
            associated_token_program: associated_token::ID,
            token_program: TOKEN_PROGRAM_ID, system_program: SYSTEM_PROGRAM_ID,
        }.to_account_metas(None),
        data: escrowq32026::instruction::Refund {}.data(),
    };
    send(&mut f.svm, &f.maker, ix)
}

fn update(f: &mut Fixture, receive: u64, expiration: i64, unauthorized: bool) -> bool {
    let actor = if unauthorized { &f.taker } else { &f.maker };
    let ix = Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Update {
            maker: actor.pubkey(), escrow: f.escrow,
        }.to_account_metas(None),
        data: escrowq32026::instruction::Update { receive, expiration }.data(),
    };
    send(&mut f.svm, actor, ix)
}

fn token_amount(svm: &LiteSVM, ata: Pubkey) -> u64 {
    let raw = svm.get_account(&ata).expect("token account exists");
    spl_token::state::Account::unpack(&raw.data).unwrap().amount
}

#[test]
fn make_stores_terms_and_locks_exact_amount() {
    let mut f = setup();
    assert!(make(&mut f, DEPOSIT, RECEIVE, EXPIRATION));
    let account = f.svm.get_account(&f.escrow).unwrap();
    let data = escrowq32026::Escrow::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(data.receive, RECEIVE);
    assert_eq!(data.expiration, EXPIRATION);
    assert_eq!(data.maker, f.maker.pubkey());
    assert_eq!(token_amount(&f.svm, f.vault), DEPOSIT);
    assert!(!make(&mut f, DEPOSIT, RECEIVE, EXPIRATION));
}

#[test]
fn take_swaps_both_tokens_atomically_and_closes_escrow() {
    let mut f = setup();
    assert!(make(&mut f, DEPOSIT, RECEIVE, EXPIRATION));
    assert!(take(&mut f));
    let maker_ata_b = associated_token::get_associated_token_address(&f.maker.pubkey(), &f.mint_b);
    let taker_ata_a = associated_token::get_associated_token_address(&f.taker.pubkey(), &f.mint_a);
    assert_eq!(token_amount(&f.svm, maker_ata_b), RECEIVE);
    assert_eq!(token_amount(&f.svm, taker_ata_a), DEPOSIT);
    assert_eq!(token_amount(&f.svm, f.taker_ata_b), 100_000_000 - RECEIVE);
    assert!(f.svm.get_account(&f.escrow).is_none());
    assert!(f.svm.get_account(&f.vault).is_none());
    assert!(!take(&mut f));
}

#[test]
fn refund_returns_locked_tokens_and_closes_escrow() {
    let mut f = setup();
    assert!(make(&mut f, DEPOSIT, RECEIVE, EXPIRATION));
    assert!(refund(&mut f));
    assert_eq!(token_amount(&f.svm, f.maker_ata_a), 100_000_000);
    assert!(f.svm.get_account(&f.escrow).is_none());
    assert!(f.svm.get_account(&f.vault).is_none());
    assert!(!refund(&mut f));
}

#[test]
fn update_changes_price_and_deadline_only_for_maker() {
    let mut f = setup();
    assert!(make(&mut f, DEPOSIT, RECEIVE, EXPIRATION));
    assert!(update(&mut f, 13_000_000, EXPIRATION + 500, false));
    assert!(!update(&mut f, 1, EXPIRATION + 600, true));
    assert!(!update(&mut f, 0, EXPIRATION + 600, false));
    let account = f.svm.get_account(&f.escrow).unwrap();
    let data = escrowq32026::Escrow::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(data.receive, 13_000_000);
    assert_eq!(data.expiration, EXPIRATION + 500);
    assert_eq!(token_amount(&f.svm, f.vault), DEPOSIT);
    assert!(take(&mut f));
    let maker_ata_b = associated_token::get_associated_token_address(&f.maker.pubkey(), &f.mint_b);
    assert_eq!(token_amount(&f.svm, maker_ata_b), 13_000_000);
}

#[test]
fn make_rejects_invalid_amounts_and_must_be_funded() {
    let mut f = setup();
    assert!(!make(&mut f, 0, RECEIVE, EXPIRATION));
    assert!(!make(&mut f, DEPOSIT, 0, EXPIRATION));
    assert!(!make(&mut f, DEPOSIT, RECEIVE, 0));
    assert!(f.svm.get_account(&f.escrow).is_none());
    assert!(make(&mut f, DEPOSIT, RECEIVE, EXPIRATION));
}
