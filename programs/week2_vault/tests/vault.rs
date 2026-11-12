use {
    anchor_lang::{
        solana_program::{instruction::Instruction, system_program::ID as SYSTEM_PROGRAM_ID},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

const USER_FUNDS: u64 = 5_000_000_000;

struct Fixture {
    svm: LiteSVM,
    user: Keypair,
    state: Pubkey,
    vault: Pubkey,
}

fn setup() -> Fixture {
    let mut svm = LiteSVM::new();
    svm.add_program(
        week2_vault::id(),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/week2_vault.so")),
    )
    .unwrap();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), USER_FUNDS).unwrap();
    let state = Pubkey::find_program_address(&[b"state", user.pubkey().as_ref()], &week2_vault::id()).0;
    let vault = Pubkey::find_program_address(&[b"vault", user.pubkey().as_ref()], &week2_vault::id()).0;
    Fixture { svm, user, state, vault }
}

fn run(f: &mut Fixture, instruction: Instruction) -> bool {
    let message = Message::new(&[instruction], Some(&f.user.pubkey()));
    let tx = Transaction::new(&[&f.user], message, f.svm.latest_blockhash());
    f.svm.send_transaction(tx).is_ok()
}

fn initialize(f: &mut Fixture) -> bool {
    let ix = Instruction {
            program_id: week2_vault::id(),
            accounts: week2_vault::accounts::Initialize {
                user: f.user.pubkey(),
                state: f.state,
                vault: f.vault,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: week2_vault::instruction::Initialize {}.data(),
        };
    run(f, ix)
}

fn deposit(f: &mut Fixture, amount: u64) -> bool {
    let ix = Instruction {
        program_id: week2_vault::id(),
        accounts: week2_vault::accounts::Deposit {
            user: f.user.pubkey(), state: f.state, vault: f.vault, system_program: SYSTEM_PROGRAM_ID,
        }.to_account_metas(None),
        data: week2_vault::instruction::Deposit { amount }.data(),
    };
    run(f, ix)
}

fn withdraw(f: &mut Fixture, amount: u64) -> bool {
    let ix = Instruction {
        program_id: week2_vault::id(),
        accounts: week2_vault::accounts::Withdraw {
            user: f.user.pubkey(), state: f.state, vault: f.vault, system_program: SYSTEM_PROGRAM_ID,
        }.to_account_metas(None),
        data: week2_vault::instruction::Withdraw { amount }.data(),
    };
    run(f, ix)
}

fn close(f: &mut Fixture) -> bool {
    let ix = Instruction {
        program_id: week2_vault::id(),
        accounts: week2_vault::accounts::Close {
            user: f.user.pubkey(), state: f.state, vault: f.vault, system_program: SYSTEM_PROGRAM_ID,
        }.to_account_metas(None),
        data: week2_vault::instruction::Close {}.data(),
    };
    run(f, ix)
}

fn vault_balance(f: &Fixture) -> u64 {
    f.svm.get_account(&f.vault).map(|a| a.lamports).unwrap_or_default()
}

#[test]
fn initializes_state_and_vault() {
    let mut f = setup();
    assert!(initialize(&mut f));
    let raw = f.svm.get_account(&f.state).expect("state exists");
    let state = week2_vault::VaultState::try_deserialize(&mut raw.data.as_slice()).unwrap();
    assert_eq!(state.owner, f.user.pubkey());
    assert!(vault_balance(&f) > 0);
    assert!(!initialize(&mut f));
}

#[test]
fn deposit_withdraw_and_close() {
    let mut f = setup();
    assert!(initialize(&mut f));
    let reserve = vault_balance(&f);
    assert!(deposit(&mut f, 1_000_000));
    assert_eq!(vault_balance(&f), reserve + 1_000_000);
    assert!(withdraw(&mut f, 400_000));
    assert_eq!(vault_balance(&f), reserve + 600_000);
    assert!(close(&mut f));
    assert!(f.svm.get_account(&f.state).is_none());
    assert_eq!(vault_balance(&f), 0);
}

#[test]
fn rejects_zero_and_excessive_amounts() {
    let mut f = setup();
    assert!(initialize(&mut f));
    assert!(!deposit(&mut f, 0));
    assert!(!withdraw(&mut f, 0));
    assert!(!withdraw(&mut f, 100));
    assert!(deposit(&mut f, 1_000_000));
    assert!(!withdraw(&mut f, 1_000_001));
}

#[test]
fn different_users_have_different_addresses() {
    let f = setup();
    let outsider = Keypair::new();
    let (other_state, _) = Pubkey::find_program_address(
        &[b"state", outsider.pubkey().as_ref()], &week2_vault::id()
    );
    assert_ne!(other_state, f.state);
}
