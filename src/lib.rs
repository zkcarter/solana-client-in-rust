use std::borrow::Borrow;
use std::path::Path;

use solana_cli_config::{Config, CONFIG_FILE};

use solana_client::{
    rpc_client::RpcClient,
    rpc_response::RpcVersionInfo,
    client_error::Result as ClientResult  
};

use solana_sdk::account_info::AccountInfo;
use solana_sdk::config::program;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{self, Transaction};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Keypair,
    signature::read_keypair_file,
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;

use borsh::{BorshDeserialize, BorshSerialize};

use std::cell::RefCell;


const PROGRAM_SO_PATH: &str = "/Users/carter/Programs/rust/example-helloworld/dist/program/helloworld.so";
const PROGRAM_KEYPAIR_PATH: &str = "/Users/carter/Programs/rust/example-helloworld/dist/program/helloworld-keypair.json";
const GREETING_SEED: &str = "gulei";

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct GreetingAccount {
    pub counter: u32,
}

pub struct Client {
    config: Config,
    connection: RpcClient,
    payer: Keypair,
    program_id: Pubkey,
}


impl Client {
    pub fn new() -> Client {
        let config = Config::load(CONFIG_FILE.as_ref().unwrap()).unwrap();
        let json_rpc_url = String::from(&config.json_rpc_url);
        println!("Get config file: {:?}", config);
        println!("Connecting to {}", config.json_rpc_url);
        Client {
            config,
            connection: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            payer: Keypair::new(),
            program_id: read_keypair_file(PROGRAM_KEYPAIR_PATH).unwrap().pubkey(),
        }
    }

    pub fn get_greeting_size(&self) -> usize {
        let x = GreetingAccount { counter: 0 };
        let size = x.try_to_vec().unwrap().len();
        println!("Greeting account size = {}", size);
        size
    }

    fn get_payer(&self) -> Keypair {
        let keypair_path = &self.config.keypair_path;
        if self.config.keypair_path.is_empty() {
            println!("Failed to create keypair from CLI config file, falling back to new random keypair");
            Keypair::new()
        } else {
            read_keypair_file(&Path::new(keypair_path)).unwrap()
        }
    }

    pub fn get_version(&self) -> ClientResult<RpcVersionInfo> {
        self.connection.get_version()
    }

    pub fn establish_payer(&mut self) {
        let mut fees: u64 = 0;
        let (_, fee_calculator) = self.connection.get_recent_blockhash().unwrap();
        let greeting_size = self.get_greeting_size();
        // Calculate the cost to fund the greeter account
        fees += self.connection.get_minimum_balance_for_rent_exemption(greeting_size).unwrap();
        // Calculate the cost of sending transactions
        fees += fee_calculator.lamports_per_signature * 100;

        self.payer = self.get_payer();

        let ref pub_key = self.payer.pubkey();
        let mut lamports = self.connection.get_balance(pub_key).unwrap();

        if lamports < fees {
            let sig = self.connection.request_airdrop(pub_key, fees - lamports);
            let _confirmed = self.connection.confirm_transaction(&sig.unwrap());
            lamports = self.connection.get_balance(pub_key).unwrap();
        }

        println!("Using account {} containing {} SOL to pay for fees",
            pub_key,
            lamports / LAMPORTS_PER_SOL
        );
    }

    pub fn check_program(&self) -> Pubkey {
        let program_info = self.connection.get_account(&self.program_id);
        if program_info.is_err() {
            if !Path::new(PROGRAM_SO_PATH).exists() {
                println!("Program needs to be deployed with `solana program deploy dist/program/helloworld.so`");
            } else {
                println!("Program needs to be built and deployed");
            }
        } else if !program_info.unwrap().executable {
            println!("Program is not executable");
        }

        println!("Using program {}", self.program_id);

        // Generat the address (public key) of a greeting account from the program so that it's easy to find later.
        let greeted_pubkey = Pubkey::create_with_seed(
            &self.payer.pubkey(),
            GREETING_SEED,
            &self.program_id).unwrap();

        // Check if the greeting account has already been created
        let greet_account = self.connection.get_account(&greeted_pubkey);
        if greet_account.is_err() {
            println!("Creating a account {} to say hello", greeted_pubkey);
            let lamports = self.connection.get_minimum_balance_for_rent_exemption(4).unwrap();
            let intruction = solana_sdk::system_instruction::create_account_with_seed(
                &self.payer.pubkey(),
                &greeted_pubkey,
                &self.payer.pubkey(),
                GREETING_SEED,
                lamports,
                4,
                &self.program_id);
            let (recent_hash, _) = self.connection.get_recent_blockhash().unwrap();
            let transaction = Transaction::new_signed_with_payer(
                &[intruction],
                Some(&self.payer.pubkey()),
                &[&self.payer],
            recent_hash);
            self.connection.send_and_confirm_transaction(&transaction);
        }
        greeted_pubkey
    }

    pub fn say_hello(&self, greeted_pubkey: &Pubkey) {
        let program_id = self.program_id.clone();
        println!("Say hello to {}, owner is {}", greeted_pubkey, program_id);
        //let signers = [&self.payer, &self.payer];
    
        let (recent_hash, _) = self.connection.get_recent_blockhash().unwrap();
        let instruction = solana_sdk::instruction::Instruction::new_with_bytes(
            program_id,
            &[0; 1],
            vec![AccountMeta::new(*greeted_pubkey, false)]);
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_hash);
        let sig = self.connection.send_and_confirm_transaction(&transaction);
        println!("Say hello: {}", sig.unwrap());
    }

    pub fn report(&self, greeted_pubkey: &Pubkey) {
        let account_info = self.connection.get_account(greeted_pubkey).unwrap();
        let greeting = GreetingAccount::try_from_slice(account_info.data.borrow());
        println!("{} has been greeted {} time(s)", greeted_pubkey, greeting.unwrap().counter);
    }
}