use {
    async_tungsten::{self, WebSocket},
    serde::{Deserialize, Serialize},
    solana_client::{
        rpc_client::{RpcClient, RpcClientError},
        transaction::Transaction,
    },
    solana_sdk::{commitment::Commitment, pubkey::Pubkey, signature::Signature},
    std::{collections::HashMap, error::Error},
};

#[derive(Debug, Serialize, Deserialize)]
struct LogMessage {
    signature: Signature,
    logs: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstructionInfo {
    program_id: Pubkey,
    accounts: Vec<Pubkey>,
}

async fn main() -> Result<(), Box<dyn Error>> {
    let url = "wss://api.mainnet-beta.solana.com";
    let raydium_program_id = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;
    let solana_client = RpcClient::new("https://api.mainnet-beta.solana.com");

    let mut websocket = connect(url).await?;
    let subscription_id = subscribe_to_logs(&mut websocket, raydium_program_id, Commitment::Finalized).await?;

    let mut seen_signatures = HashMap::new();

    while let Some(message) = websocket.recv().await {
        let message = serde_json::from_value::<LogMessage>(message)?;
        let signature = message.signature;

        if seen_signatures.contains_key(&signature) {
            continue;
        }
        seen_signatures.insert(signature, true);

        let transaction = get_transaction(&solana_client, signature).await?;
        let tokens = extract_tokens(&transaction, raydium_program_id)?;
        print_table(tokens);
    }

    Ok(())
}

async fn subscribe_to_logs(
    websocket: &mut WebSocket,
    program_id: Pubkey,
    commitment: Commitment,
) -> Result<u64, RpcClientError> {
    websocket
        .send_text(
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "solana_subscribeLogs",
                "params": {
                    "filter": {"mentions": program_id},
                    "commitment": commitment.to_string(),
                },
                "id": 1,
            }),
        )
        .await?;

    let response = websocket.recv().await?;
    let response_value = serde_json::from_value::<Value>(response)?;
    response_value["result"]
        .as_u64()
        .ok_or_else(|| RpcClientError::SubscriptionError)
}

async fn get_transaction(client: &RpcClient, signature: Signature) -> Result<Transaction, RpcClientError> {
    client
        .get_transaction(signature, Some("jsonParsed"), None)
        .await
}

fn extract_tokens(transaction: &Transaction, program_id: Pubkey) -> Result<Vec<Pubkey>, Box<dyn Error>> {
    let mut tokens = Vec::new();
    for instruction in transaction.message.instructions.iter() {
        if instruction.program_id == program_id {
            let accounts = &instruction.accounts;
            let token0 = accounts[8];
            let token1 = accounts[9];
            tokens.push(token0);
            tokens.push(token1);
        }
    }
    Ok(tokens)
}

fn print_table(tokens: Vec<Pubkey>) {
    println!("============NEW POOL DETECTED====================");
    let header = vec!["Token_Index", "Account Public Key"];
    println!("│ {:^15} │ {:^40} │", header[0], header[1]);
    println!("|----------|----------------------------------------|");
    for (i, token) in tokens.iter().enumerate() {
        println!("│ {:^15} │ {:^40} │", format!("Token{}", i + 1), token);
    }
}
