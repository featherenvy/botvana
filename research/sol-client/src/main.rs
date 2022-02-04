use std::{env::var, str::FromStr};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    commitment_config::CommitmentConfig
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn establish_connection(rpc_url: String) -> Result<RpcClient> {
    Ok(RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    ))
}

fn main() {
    let client = establish_connection(var("RPC_URL").expect("Missing RPC_URL")).unwrap();

    let pubkey = Pubkey::from_str("8WZrmdpLckptiVKd2fPHPjewRVYQGQkjxi9vzRYG1sfs").unwrap();

    let account = client.get_account_data(&pubkey);

    println!("{account:?}");

    let account = client.get_program_accounts(&pubkey);

    println!("{account:?}");
}
