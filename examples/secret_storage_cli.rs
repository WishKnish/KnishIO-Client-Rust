use knishio_client::storage::{envelope, EncryptedSecretPayload, SecretStorageMetadata};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: secret_storage_cli <seal|open> [args...]");
        std::process::exit(1);
    }
    match args[1].as_str() {
        "seal" | "seal-recovery" => {
            if args.len() < 5 {
                eprintln!("Usage: secret_storage_cli seal <passphrase> <secret> <bundle_hash> [label]");
                std::process::exit(1);
            }
            let passphrase = &args[2];
            let secret = &args[3];
            let bundle_hash = &args[4];
            let label = args.get(5).cloned().filter(|s| !s.is_empty());
            let metadata = SecretStorageMetadata {
                bundle_hash: bundle_hash.clone(),
                label,
                created_at: 1700000000000,
                hardware_backed: false,
                provider_type: "aes-gcm".to_string(),
            };
            let payload = envelope::seal(secret, passphrase, metadata).expect("seal failed");
            println!("{}", serde_json::to_string(&payload).expect("json failed"));
        }
        "open" => {
            if args.len() < 4 {
                eprintln!("Usage: secret_storage_cli open <passphrase> <payload_json>");
                std::process::exit(1);
            }
            let passphrase = &args[2];
            let payload_json = &args[3];
            let payload: EncryptedSecretPayload =
                serde_json::from_str(payload_json).expect("parse payload failed");
            let plaintext = envelope::open(&payload, passphrase).expect("open failed");
            println!("{}", String::from_utf8(plaintext.to_vec()).expect("utf8 failed"));
        }
        cmd => {
            eprintln!("Unknown command: {}", cmd);
            std::process::exit(1);
        }
    }
}
