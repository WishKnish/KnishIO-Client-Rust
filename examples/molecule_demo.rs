use knishio_client::{Atom, Isotope, Wallet, Molecule, types::MetaItem, crypto::shake256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== KnishIO Rust SDK - Molecule Demo ===\n");
    
    // Create wallets
    println!("1. Creating wallets...");
    let secret = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    
    let mut sender_wallet = Wallet::create(Some(secret), None, "TEST", None, None)?;
    sender_wallet.balance = 1000.0;
    println!("   Sender wallet: {}", sender_wallet.address.as_ref().unwrap());
    println!("   Sender balance: {}", sender_wallet.balance);
    
    let recipient_wallet = Wallet::create(Some("different-secret"), None, "TEST", None, None)?;
    println!("   Recipient wallet: {}", recipient_wallet.address.as_ref().unwrap());
    
    // Create molecule
    println!("\n2. Creating molecule for value transfer...");
    let mut molecule = Molecule::with_params(
        Some(secret.to_string()),
        None,
        Some(sender_wallet.clone()),
        None,
        None,
        None,
    );
    
    // Initialize value transfer
    let transfer_amount = 250.0;
    molecule.init_value(&recipient_wallet, transfer_amount)?;
    println!("   Transfer amount: {}", transfer_amount);
    println!("   Atoms created: {}", molecule.atoms.len());
    
    // Show atoms
    println!("\n3. Molecule atoms:");
    for (i, atom) in molecule.atoms.iter().enumerate() {
        println!("   Atom {}: {} isotope, value: {:?}, address: {}", 
                 i, 
                 atom.isotope.as_str(), 
                 atom.value, 
                 &atom.wallet_address[0..8]);
    }
    
    // Sign the molecule
    println!("\n4. Signing molecule...");
    let last_position = molecule.sign(None, false, true)?;
    println!("   Molecular hash: {}", molecule.molecular_hash.as_ref().unwrap());
    println!("   Last position: {:?}", last_position);
    
    // Show signature fragments
    println!("\n5. OTS signature fragments:");
    for (i, atom) in molecule.atoms.iter().enumerate() {
        if let Some(ref fragment) = atom.ots_fragment {
            println!("   Atom {} OTS fragment: {}...", i, &fragment[0..20.min(fragment.len())]);
        }
    }
    
    // Test molecular hash enumeration and normalization
    println!("\n6. Testing cryptographic functions...");
    let hash = molecule.molecular_hash.as_ref().unwrap();
    let enumerated = Molecule::enumerate(hash);
    let normalized = Molecule::normalize(enumerated.clone());
    
    println!("   Original hash: {}", hash);
    println!("   Enumerated (first 10): {:?}", &enumerated[0..10]);
    println!("   Normalized (first 10): {:?}", &normalized[0..10]);
    println!("   Normalized sum: {}", normalized.iter().map(|&x| x as i32).sum::<i32>());
    
    // Test serialization
    println!("\n7. Testing JSON serialization...");
    let json = serde_json::to_string_pretty(&molecule)?;
    println!("   JSON length: {} characters", json.len());
    println!("   Contains molecular hash: {}", json.contains("molecularHash"));
    println!("   Contains atoms: {}", json.contains("atoms"));
    
    // Test deserialization
    let deserialized = Molecule::json_to_object(&json)?;
    println!("   Deserialized atoms count: {}", deserialized.atoms.len());
    println!("   Molecular hash matches: {}", 
             deserialized.molecular_hash == molecule.molecular_hash);
    
    // Create a metadata molecule
    println!("\n8. Creating metadata molecule...");
    let mut meta_molecule = Molecule::with_params(
        Some(secret.to_string()),
        None,
        Some(sender_wallet),
        None,
        None,
        None,
    );
    
    let metadata = vec![
        MetaItem::new("name", "Test Document"),
        MetaItem::new("type", "document"),
        MetaItem::new("version", "1.0"),
    ];
    
    meta_molecule.init_meta(metadata, "document", "doc123", None)?;
    println!("   Metadata molecule atoms: {}", meta_molecule.atoms.len());
    println!("   First atom isotope: {}", meta_molecule.atoms[0].isotope.as_str());
    
    // Test validation
    println!("\n9. Testing molecule validation...");
    let is_valid = molecule.check(None)?;
    println!("   Molecule is valid: {}", is_valid);
    
    println!("\n=== Demo completed successfully! ===");
    
    Ok(())
}