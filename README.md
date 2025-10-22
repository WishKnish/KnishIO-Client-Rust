<div style="text-align:center">
  <img src="https://raw.githubusercontent.com/WishKnish/KnishIO-Technical-Whitepaper/master/KnishIO-Logo.png" alt="Knish.IO: Post-Blockchain Platform" />
</div>
<div style="text-align:center">info@wishknish.com | https://wishknish.com</div>

# Knish.IO Rust Client SDK

This is the official Rust implementation of the Knish.IO client SDK. Its purpose is to expose libraries for building and signing Knish.IO Molecules, composing Atoms, generating Wallets, and much more with native performance, memory safety, and quantum-resistant security.

## Installation

The SDK can be installed via Cargo:

```bash
# Add to Cargo.toml
[dependencies]
knishio-client = "0.1.0"

# Or install from the command line
cargo add knishio-client
```

**Requirements:**
- Rust 1.70 or higher
- Cargo for dependency management
- Required dependencies: `serde`, `sha3`, `hex`, `base64`, `chrono`, `rand`

After installation, import the SDK in your project:

```rust
use knishio_client::{KnishIOClient, Molecule, Wallet, Atom};
use knishio_client::crypto::shake256;
use knishio_client::types::{MetaItem, Isotope};
```

## Basic Usage

The purpose of the Knish.IO SDK is to expose various ledger functions to new or existing applications.

There are two ways to take advantage of these functions:

1. The easy way: use the `KnishIOClient` wrapper struct

2. The granular way: build `Atom` and `Molecule` instances and broadcast GraphQL messages yourself

This document will explain both ways.

## The Easy Way: KnishIOClient Wrapper

1. Include the wrapper struct in your application code:
   ```rust
   use knishio_client::KnishIOClient;
   ```

2. Instantiate the client with your node URI:
   ```rust
   let client = KnishIOClient::new(
       vec!["https://some.knishio.validator.node.url/graphql".to_string()],
       Some("my-cell-slug".to_string())
   )?;
   ```

3. Set your secret for authentication:
   ```rust
   client.set_secret("myTopSecretCode")?;

   // Note: The Rust SDK uses stored secret for cryptographic operations
   // This is equivalent to the JavaScript SDK's await client.requestAuthToken()
   ```

   (**Note:** The `secret` parameter can be a salted combination of username + password, a biometric hash, an existing user identifier from an external authentication process, for example)

4. Begin using `client` to trigger commands described below...

### KnishIOClient Methods

- Query metadata for a **Wallet Bundle**. Omit the `bundle_hash` parameter to query your own Wallet Bundle:
  ```rust
  let response = client.query_bundle(
      Some("c47e20f99df190e418f0cc5ddfa2791e9ccc4eb297cfa21bd317dc0f98313b1d")
  ).await?;

  if response.success() {
      println!("{:?}", response.data()); // Raw Metadata
  }
  ```

- Query metadata for a **Meta Asset**:

  ```rust
  let result = client.query_meta(
      "Vehicle",      // meta_type
      Some("CAR123"), // meta_id
      Some("LicensePlate"), // key
      Some("1H17P"),  // value
      true            // latest
  ).await?;

  println!("{:?}", result); // Raw Metadata
  ```

- Writing new metadata for a **Meta Asset**:

  ```rust
  use knishio_client::types::MetaItem;

  let metadata = vec![
      MetaItem::new("type", "fire"),
      MetaItem::new("weaknesses", "rock,water,electric"),
      MetaItem::new("immunities", "ground"),
      MetaItem::new("hp", "78"),
      MetaItem::new("attack", "84"),
  ];

  let response = client.create_meta(
      "Pokemon",      // meta_type
      "Charizard",    // meta_id
      metadata
  ).await?;

  if response.success() {
      println!("Metadata created successfully!");
  }

  println!("{:?}", response.data()); // Raw response
  ```

- Query Wallets associated with a Wallet Bundle:

  ```rust
  let wallets = client.query_wallets(
      "c47e20f99df190e418f0cc5ddfa2791e9ccc4eb297cfa21bd317dc0f98313b1d", // bundle_hash
      Some("FOO"), // token (optional)
      true         // unspent
  ).await?;

  println!("{:?}", wallets); // Raw response
  ```

- Declaring new **Wallets**:

  (**Note:** If Tokens are sent to undeclared Wallets, **Shadow Wallets** will be used (placeholder
  Wallets that can receive, but cannot send) to store tokens until they are claimed.)

  ```rust
  let response = client.create_wallet("FOO").await?; // Token Slug for the wallet we are declaring

  if response.success() {
      println!("Wallet created successfully!");
  }

  println!("{:?}", response.data()); // Raw response
  ```

- Issuing new **Tokens**:

  ```rust
  let token_meta = vec![
      MetaItem::new("name", "CrazyCoin"), // Public name for the token
      MetaItem::new("fungibility", "fungible"), // Fungibility style
      MetaItem::new("supply", "limited"), // Supply style
      MetaItem::new("decimals", "2"), // Decimal places
  ];

  let response = client.create_token(
      "CRZY",        // Token slug (ticker symbol)
      100000000.0,   // Initial amount to issue
      token_meta,
      vec![],        // units (optional, for stackable tokens)
      None           // batch_id (optional, for stackable tokens)
  ).await?;

  if response.success() {
      println!("Token created successfully!");
  }

  println!("{:?}", response.data()); // Raw response
  ```

- Transferring **Tokens** to other users:

  ```rust
  let response = client.transfer_token(
      "7bf38257401eb3b0f20cabf5e6cf3f14c76760386473b220d95fa1c38642b61d", // Recipient's bundle hash
      "CRZY",    // Token slug
      100.0,     // Amount
      vec![],    // units (optional, for stackable tokens)
      None       // batch_id (optional, for stackable tokens)
  ).await?;

  if response.success() {
      println!("Token transferred successfully!");
  }

  println!("{:?}", response.data()); // Raw response
  ```

- Creating a new **Rule**:

  ```rust
  let rule = vec![
      // Rule definition
  ];

  let response = client.create_rule(
      "MyMetaType",  // meta_type
      "MyMetaId",    // meta_id
      rule,
      None           // policy (optional)
  ).await?;

  if response.success() {
      println!("Rule created successfully!");
  }

  println!("{:?}", response.data()); // Raw response
  ```

- Querying **Atoms**:

  ```rust
  let response = client.query_atom(
      Some("molecular_hash_here"),
      Some("bundle_hash_here"),
      Some(Isotope::V),
      Some("CRZY"),
      true,  // latest
      15,    // limit
      1      // offset
  ).await?;

  println!("{:?}", response.data()); // Raw response
  ```

- Working with **Buffer Tokens**:

  ```rust
  // Deposit to buffer
  let deposit_response = client.deposit_buffer_token(
      "CRZY",          // token_slug
      100.0,           // amount
      vec![("OTHER_TOKEN".to_string(), 0.5)] // trade_rates
  ).await?;

  // Withdraw from buffer
  let withdraw_response = client.withdraw_buffer_token(
      "CRZY",  // token_slug
      50.0     // amount
  ).await?;

  println!("{:?} {:?}", deposit_response.data(), withdraw_response.data());
  ```

## Advanced Usage: Working with Molecules

For more granular control, you can work directly with Molecules:

- Create a new Molecule:
  ```rust
  use knishio_client::Molecule;

  let mut molecule = Molecule::new(
      Some("secret".to_string()),
      None,                    // bundle
      Some(source_wallet),     // source_wallet
      None,                    // remainder_wallet
      Some("cell_slug".to_string()),
      None                     // version
  );
  ```

- Create a custom Mutation:
  ```rust
  use knishio_client::mutation::MutationProposeMolecule;

  let mutation = MutationProposeMolecule::new(molecule);
  ```

- Sign and check a Molecule:
  ```rust
  molecule.sign(None, false, true)?;

  if molecule.check()? {
      println!("Molecule validation passed!");
  } else {
      println!("Molecule validation failed!");
  }
  ```

- Execute a custom Query or Mutation:
  ```rust
  let response = client.execute_query(mutation).await?;

  if response.success() {
      println!("Molecule executed successfully!");
  }
  ```

## The Hard Way: DIY Everything

This method involves individually building Atoms and Molecules, triggering the signature and validation processes, and communicating the resulting signed Molecule mutation or Query to a Knish.IO node via GraphQL.

1. Include the relevant structures in your application code:
    ```rust
    use knishio_client::{Molecule, Wallet, Atom};
    use knishio_client::crypto;
    use knishio_client::types::{Isotope, MetaItem};
    ```

2. Generate a 2048-symbol hexadecimal secret, either randomly, or via hashing login + password + salt, OAuth secret ID, biometric ID, or any other static value.

3. (optional) Initialize a signing wallet with:
   ```rust
   let wallet = Wallet::create(
       Some("secret"),
       None,              // bundle (optional)
       "USER",            // token
       None,              // position (optional)
       None               // characters (optional)
   )?;
   ```

   **WARNING 1:** If ContinuID is enabled on the node, you will need to use a specific wallet, and therefore will first need to query the node to retrieve the `position` for that wallet.

   **WARNING 2:** The Knish.IO protocol mandates that all C and M transactions be signed with a `USER` token wallet.

4. Build your molecule with:
   ```rust
   let mut molecule = Molecule::new(
       Some("secret".to_string()),
       None,                    // bundle (optional)
       Some(source_wallet),     // source_wallet (optional)
       None,                    // remainder_wallet (optional)
       Some("cell_slug".to_string()), // cell_slug (optional)
       None                     // version (optional)
   );
   ```

5. Either use one of the shortcut methods provided by the `Molecule` struct (which will build `Atom` instances for you), or create `Atom` instances yourself.

   DIY example:
    ```rust
    // This example records a new Wallet on the ledger

    // Define metadata for our new wallet
    let new_wallet_meta = vec![
        MetaItem::new("address", &new_wallet.address),
        MetaItem::new("token", &new_wallet.token),
        MetaItem::new("bundle", &new_wallet.bundle),
        MetaItem::new("position", &new_wallet.position.unwrap_or_default()),
        MetaItem::new("batchId", &new_wallet.batch_id.unwrap_or_default()),
    ];

    // Build the C isotope atom
    let wallet_creation_atom = Atom::new(
        &source_wallet.position.unwrap(),
        &source_wallet.address,
        Isotope::C,
        &source_wallet.token
    );
    wallet_creation_atom.meta_type = Some("wallet".to_string());
    wallet_creation_atom.meta_id = Some(new_wallet.address.clone());
    wallet_creation_atom.meta = Some(new_wallet_meta);
    wallet_creation_atom.index = Some(molecule.generate_index());

    // Add the atom to our molecule
    molecule.add_atom(wallet_creation_atom);

    // Adding a ContinuID / remainder atom
    molecule.add_continuid_atom()?;
    ```

   Molecule shortcut method example:
    ```rust
    // This example commits metadata to some Meta Asset

    // Defining our metadata
    let metadata = vec![
        MetaItem::new("foo", "Foo"),
        MetaItem::new("bar", "Bar"),
    ];

    molecule.init_meta(
        metadata,
        "MyMetaType",
        "MetaId123",
        None  // policy (optional)
    )?;
    ```

6. Sign the molecule with the stored user secret:
    ```rust
    molecule.sign(None, false, true)?;
    ```

7. Make sure everything checks out by verifying the molecule:
    ```rust
    match molecule.check() {
        Ok(true) => {
            println!("Molecule validation passed!");
        }
        Ok(false) => {
            println!("Molecule validation failed!");
        }
        Err(e) => {
            eprintln!("Molecule check error: {:?}", e);
        }
    }
    ```

8. Broadcast the molecule to a Knish.IO node:
    ```rust
    use knishio_client::mutation::MutationProposeMolecule;

    // Build our mutation object
    let mutation = MutationProposeMolecule::new(molecule);

    // Send the mutation to the node and get a response
    let response = client.execute_mutation(mutation).await?;
    ```

9. Inspect the response...
    ```rust
    // For basic queries, we look at the data property:
    println!("{:?}", response.data());

    // For mutations, check if the molecule was accepted by the ledger:
    println!("{}", if response.success() { "Success" } else { "Failed" });

    // We can also check the reason for rejection
    println!("{:?}", response.reason());

    // Some queries may also produce a payload, with additional data:
    println!("{:?}", response.payload());
    ```

   Payloads are provided by responses to the following queries:
    1. `QueryBalance` and `QueryContinuId` -> returns a `Wallet` instance
    2. `QueryWalletList` -> returns a list of `Wallet` instances
    3. `MutationProposeMolecule`, `MutationRequestAuthorization`, `MutationCreateIdentifier`, `MutationLinkIdentifier`, `MutationClaimShadowWallet`, `MutationCreateToken`, `MutationRequestTokens`, and `MutationTransferTokens` -> returns molecule metadata

## Getting Help

Knish.IO is under active development, and our team is ready to assist with integration questions. The best way to seek help is to stop by our [Telegram Support Channel](https://t.me/wishknish). You can also [send us a contact request](https://knish.io/contact) via our website.