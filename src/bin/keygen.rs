use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::fs;

fn main() {
    let keypair = Keypair::new();
    let bytes = keypair.to_bytes();
    let json = serde_json::to_string(&bytes.to_vec()).unwrap();
    fs::write("keypair.json", &json).unwrap();
    println!("Public key: {}", keypair.pubkey());
    println!("Keypair saved to keypair.json");
}
