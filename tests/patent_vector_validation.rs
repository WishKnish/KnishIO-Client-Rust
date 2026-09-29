//! Patent Appendix B: Cross-platform test vector validation
//!
//! Validates the canonical patent test vectors against the Rust SDK crypto functions.
//! These vectors provide reduction-to-practice evidence for patent claims 1-2, 4-5, 8, 12-14, 21.
//!
//! Run: cargo test --test patent_vector_validation

// cycle 127: the suite below reads the monorepo shared fixtures (include_str!), which a
// standalone checkout of this SDK lacks; build.rs sets `has_shared_fixtures` when they exist.
// Without them the suite is not compiled, and the ignored test below says so in the report
// instead of the binary silently running 0 tests.
#[cfg(not(has_shared_fixtures))]
#[test]
#[ignore = "../shared-test-results absent: patent vector suite not compiled in a standalone checkout"]
fn shared_fixtures_absent() {}

#[allow(dead_code)] // this suite reads only PHASE_B_VECTORS_JSON of the shared fixture file
#[path = "fixtures/mod.rs"]
mod fixtures;

/// Phase B vectors (contract 9.1 replenish, 9.2 stackable fusion, 9.6 buffer withdraw, create_token
/// units), read from the frozen fixture mirror so they also run in a standalone checkout; the gated
/// suite below pins the mirror to the master. Every built molecule must also pass the SDK's own check.
mod phase_b_vectors {
    use super::fixtures::PHASE_B_VECTORS_JSON;
    use knishio_client::types::MoleculeFromJsonOptions;
    use knishio_client::{generate_bundle_hash, generate_secret, Atom, Isotope, KnishIOClient, KnishIOError, Molecule, TokenUnit, Wallet};
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const SECRET: &str = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    const POSITION: &str = "1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b";
    const ZERO_BUNDLE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn section(name: &str) -> TestResult<Vec<Value>> {
        let vectors: Value = serde_json::from_str(PHASE_B_VECTORS_JSON)?;
        Ok(vectors[name]["tests"].as_array().ok_or("tests array")?.clone())
    }

    fn meta<'a>(atom: &'a Atom, key: &str) -> Option<&'a str> {
        atom.meta.iter().find(|m| m.key == key).map(|m| m.value.as_str())
    }

    /// The unit ids of an atom's `tokenUnits` meta; an absent meta is the empty list.
    fn unit_ids(atom: &Atom) -> TestResult<Vec<String>> {
        let units: Vec<Value> = match meta(atom, "tokenUnits") {
            Some(json) => serde_json::from_str(json)?,
            None => Vec::new(),
        };
        units.iter().map(|unit| Ok(unit[0].as_str().ok_or("unit id")?.to_string())).collect()
    }

    fn strings(value: &Value) -> TestResult<Vec<String>> {
        value.as_array().ok_or("string array")?.iter().map(|v| Ok(v.as_str().ok_or("string")?.to_string())).collect()
    }

    fn isotopes(molecule: &Molecule) -> Vec<String> {
        molecule.atoms.iter().map(|a| a.isotope.as_str().to_string()).collect()
    }

    fn value_sum(molecule: &Molecule) -> TestResult<i128> {
        molecule.atoms.iter()
            .filter(|a| matches!(a.isotope, Isotope::V | Isotope::B | Isotope::F))
            .map(|a| Ok(a.value.as_deref().ok_or("value")?.parse::<i128>()?))
            .sum()
    }

    fn units(ids: &[String]) -> Vec<TokenUnit> {
        ids.iter().map(|id| TokenUnit::new(id.clone(), id.clone(), None)).collect()
    }

    fn user_source() -> TestResult<Wallet> {
        Ok(Wallet::create(Some(SECRET), None, "USER", Some(POSITION), None, None)?)
    }

    #[test]
    fn token_replenish_vectors() -> TestResult {
        for test in section("token_replenish")? {
            let name = test["name"].as_str().unwrap_or_default();
            let token = test["token"].as_str().ok_or("token")?;
            let new_units: Vec<TokenUnit> = test["units"].as_array().ok_or("units")?.iter()
                .map(|triple| -> TestResult<TokenUnit> {
                    Ok(TokenUnit::new(
                        triple[0].as_str().ok_or("id")?.to_string(),
                        triple[1].as_str().ok_or("name")?.to_string(),
                        triple[2].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                    ))
                })
                .collect::<TestResult<_>>()?;

            for batch_id in [None, Some("credited-batch")] {
                let mut credited = Wallet::create(Some(SECRET), None, token, None, None, None)?;
                credited.batch_id = batch_id.map(str::to_string);
                let mut molecule = Molecule::with_params(
                    Some(SECRET.to_string()), Some(generate_bundle_hash(SECRET)), Some(user_source()?), None, None, None,
                );
                molecule.replenish_token(&credited, test["amount"].as_f64().unwrap_or(0.0), new_units.clone())
                    .map_err(|e| format!("[{name}] replenish_token: {e}"))?;

                assert_eq!(isotopes(&molecule), strings(&test["expectedIsotopes"])?, "[{name}] isotopes");
                let c = &molecule.atoms[0];
                assert_eq!(c.token, "USER", "[{name}] the C atom is signed by the USER wallet");
                assert_eq!(c.position, POSITION, "[{name}] signed at the source position");
                assert_eq!(c.value.as_deref(), test["expectedCValue"].as_str(), "[{name}] C value");
                assert_eq!(c.meta_type.as_deref(), test["expectedMetaType"].as_str(), "[{name}] metaType");
                assert_eq!(c.meta_id.as_deref(), test["expectedMetaId"].as_str(), "[{name}] metaId");
                assert_eq!(meta(c, "action"), test["expectedAction"].as_str(), "[{name}] action");
                assert_eq!(c.batch_id, credited.batch_id, "[{name}] the atom carries the credited wallet's batch id");

                let mut expected_keys = vec!["action", "address", "position", "pubkey"];
                if batch_id.is_some() {
                    expected_keys.push("batchId");
                }
                let expected_unit_ids = &test["expectedTokenUnitIds"];
                if !expected_unit_ids.is_null() {
                    expected_keys.push("tokenUnits");
                }
                let keys: Vec<&str> = c.meta.iter().map(|m| m.key.as_str()).collect();
                assert_eq!(keys, expected_keys, "[{name}] C metas and their order");
                assert_eq!(meta(c, "address"), credited.address.as_deref(), "[{name}] credited address");
                assert_eq!(meta(c, "position"), credited.position.as_deref(), "[{name}] credited position");
                assert_eq!(meta(c, "pubkey"), credited.pubkey.as_deref(), "[{name}] credited pubkey");
                assert_eq!(meta(c, "batchId"), batch_id, "[{name}] batchId meta");
                if expected_unit_ids.is_null() {
                    assert!(unit_ids(c)?.is_empty(), "[{name}] a fungible replenish carries no tokenUnits");
                } else {
                    assert_eq!(unit_ids(c)?, strings(expected_unit_ids)?, "[{name}] tokenUnits ids");
                    let compact = serde_json::to_string(&test["units"])?;
                    assert_eq!(meta(c, "tokenUnits"), Some(compact.as_str()), "[{name}] compact triples, as JS JSON.stringify");
                }
                assert_eq!(molecule.atoms[1].token, "USER", "[{name}] the I atom continues the USER chain");

                molecule.sign(None, false, true).map_err(|e| format!("[{name}] sign: {e}"))?;
                molecule.check(None).map_err(|e| format!("[{name}] check: {e}"))?;
            }
        }
        Ok(())
    }

    #[test]
    fn stackable_fusion_vectors() -> TestResult {
        for test in section("stackable_fusion_conservation")? {
            let name = test["name"].as_str().unwrap_or_default();
            let source_units = strings(&test["sourceUnits"])?;
            let fuse = strings(&test["fuse"])?;
            let new_unit_id = test["newUnitId"].as_str().ok_or("newUnitId")?;

            for source_batch in [None, Some("source-batch")] {
                let mut source = Wallet::create(Some(SECRET), None, "STK", Some(POSITION), None, None)?;
                source.token_units = units(&source_units);
                source.balance = source_units.len().to_string();
                source.batch_id = source_batch.map(str::to_string);
                let mut recipient = Wallet::create(Some(SECRET), None, "STK", None, None, None)?;
                recipient.init_batch_id(Some(&source), false);
                recipient.token_units = vec![TokenUnit::new(new_unit_id.to_string(), new_unit_id.to_string(), None)];

                let mut molecule = Molecule::with_params(
                    Some(SECRET.to_string()), Some(generate_bundle_hash(SECRET)), Some(source.clone()), None, None, None,
                );
                let result = molecule.fuse_token(fuse.clone(), &recipient);

                if test["mustReject"].as_bool() == Some(true) {
                    let expected = test["expectedErrorContains"].as_str().ok_or("expectedErrorContains")?;
                    assert!(
                        matches!(&result, Err(KnishIOError::TransferBalanceReason(m)) if m.contains(expected)),
                        "[{name}] must be refused with {expected:?}, got {result:?}"
                    );
                    assert!(molecule.atoms.is_empty(), "[{name}] a refused fusion adds no atom");
                    continue;
                }
                result.map_err(|e| format!("[{name}] fuse_token: {e}"))?;

                assert_eq!(isotopes(&molecule), strings(&test["expectedIsotopes"])?, "[{name}] isotopes");
                let [src, burn, fusion, remainder] = &molecule.atoms[..] else { return Err(format!("[{name}] four atoms").into()) };
                let own_bundle = generate_bundle_hash(SECRET);

                assert_eq!(src.value.as_deref(), test["expectedSourceValue"].as_str(), "[{name}] source value");
                assert_eq!(src.position, POSITION, "[{name}] signed by S at its position");
                assert_eq!(unit_ids(src)?, strings(&test["expectedSourceUnitIds"])?, "[{name}] source (SENT) units");

                assert_eq!(burn.value.as_deref(), test["expectedBurnValue"].as_str(), "[{name}] burn value");
                assert_eq!((burn.meta_type.as_deref(), burn.meta_id.as_deref()), (Some("walletBundle"), Some(ZERO_BUNDLE)), "[{name}] burn target");
                assert_eq!(unit_ids(burn)?, strings(&test["expectedBurnUnitIds"])?, "[{name}] burned units");

                assert_eq!(fusion.value.as_deref(), test["expectedFusionValue"].as_str(), "[{name}] F value");
                assert_eq!((fusion.meta_type.as_deref(), fusion.meta_id.as_deref()), (Some("walletBundle"), Some(own_bundle.as_str())), "[{name}] F recipient");
                let new_units: Vec<Value> = serde_json::from_str(meta(fusion, "tokenUnits").ok_or("F tokenUnits")?)?;
                assert_eq!(new_units.len(), 1, "[{name}] F creates exactly one unit");
                assert_eq!(new_units[0][0], new_unit_id, "[{name}] F unit id");
                let fused_triples = new_units[0][2]["fusedTokenUnits"].as_array().ok_or("fusedTokenUnits")?;
                let fused_ids = strings(&Value::from(fused_triples.iter().map(|t| t[0].clone()).collect::<Vec<_>>()))?;
                assert_eq!(fused_ids, strings(&test["expectedFusedTokenUnitIds"])?, "[{name}] fusedTokenUnits ids");
                for triple in fused_triples {
                    assert_eq!(triple, &serde_json::json!([triple[0], triple[0], {}]), "[{name}] full source triples");
                }

                assert_eq!(remainder.value.as_deref(), test["expectedRemainderValue"].as_str(), "[{name}] remainder value");
                assert_eq!((remainder.meta_type.as_deref(), remainder.meta_id.as_deref()), (Some("walletBundle"), Some(own_bundle.as_str())), "[{name}] remainder bundle");
                assert_eq!(unit_ids(remainder)?, strings(&test["expectedRemainderUnitIds"])?, "[{name}] kept units");
                assert_ne!(remainder.position, POSITION, "[{name}] the remainder is a fresh position");

                assert_eq!(value_sum(&molecule)?.to_string(), test["expectedSum"].as_str().unwrap_or_default(), "[{name}] V+F sum");

                // Batch ids: none anywhere without one on S; otherwise the remainder keeps S's,
                // and the burn and F atoms carry fresh ones (every V atom needs one).
                match source_batch {
                    None => assert!(molecule.atoms.iter().all(|a| a.batch_id.is_none()), "[{name}] no batch ids"),
                    Some(batch) => {
                        assert_eq!(src.batch_id.as_deref(), Some(batch), "[{name}] source batch");
                        assert_eq!(remainder.batch_id.as_deref(), Some(batch), "[{name}] remainder keeps S's batch");
                        for (label, atom) in [("burn", burn), ("F", fusion)] {
                            assert!(atom.batch_id.is_some() && atom.batch_id.as_deref() != Some(batch), "[{name}] {label} gets a fresh batch id");
                        }
                    }
                }

                molecule.sign(None, false, true).map_err(|e| format!("[{name}] sign: {e}"))?;
                molecule.check(Some(&source)).map_err(|e| format!("[{name}] check (batch {source_batch:?}): {e}"))?;
            }
        }
        Ok(())
    }

    #[test]
    fn buffer_withdraw_fresh_remainder_vectors() -> TestResult {
        let bundle = generate_bundle_hash(SECRET);
        for test in section("buffer_withdraw_fresh_remainder")? {
            let name = test["name"].as_str().unwrap_or_default();
            let mut source = Wallet::create(Some(SECRET), None, "BUFTOK", Some(POSITION), None, None)?;
            source.balance = test["sourceBalance"].as_i64().ok_or("sourceBalance")?.to_string();

            // No remainder wallet set: the builder derives the fresh remainder itself.
            let mut molecule = Molecule::new();
            molecule.secret = Some(SECRET.to_string());
            molecule.bundle = Some(bundle.clone());
            molecule.source_wallet = Some(source.clone());
            let recipients = HashMap::from([(bundle.clone(), test["amount"].as_f64().ok_or("amount")?)]);
            molecule.init_withdraw_buffer(recipients).map_err(|e| format!("[{name}] init_withdraw_buffer: {e}"))?;

            assert_eq!(isotopes(&molecule), strings(&test["expectedIsotopes"])?, "[{name}] isotopes");
            let [src, recipient, remainder] = &molecule.atoms[..] else { return Err(format!("[{name}] three atoms").into()) };
            assert_eq!(src.value.as_deref(), test["expectedSourceValue"].as_str(), "[{name}] source B value");
            assert_eq!(src.position, POSITION, "[{name}] signed by the buffer wallet");
            assert_eq!(recipient.value.as_deref(), test["expectedRecipientValue"].as_str(), "[{name}] recipient V value");
            assert!(recipient.position.is_empty() && recipient.wallet_address.is_empty(), "[{name}] the recipient is addressless");
            assert_eq!(recipient.meta_id.as_deref(), Some(bundle.as_str()), "[{name}] recipient bundle");
            assert_eq!(remainder.value.as_deref(), test["expectedRemainderValue"].as_str(), "[{name}] remainder B value");
            assert_eq!(remainder.meta_type.as_deref(), Some("walletBundle"), "[{name}] remainder metaType");
            if test["expectedRemainderPositionDistinctFromSource"].as_bool() == Some(true) {
                assert!(!remainder.position.is_empty() && remainder.position != POSITION, "[{name}] fresh remainder position");
            }
            assert_eq!(value_sum(&molecule)?.to_string(), test["expectedSum"].as_str().unwrap_or_default(), "[{name}] V+B sum");
            assert!(molecule.atoms.iter().all(|a| a.isotope != Isotope::I), "[{name}] no I atom");

            molecule.sign(None, false, true).map_err(|e| format!("[{name}] sign: {e}"))?;
            molecule.check(Some(&source)).map_err(|e| format!("[{name}] check: {e}"))?;

            // A remainder at the source's own position is refused before any atom is built.
            let mut at_source = Molecule::new();
            at_source.secret = Some(SECRET.to_string());
            at_source.source_wallet = Some(source.clone());
            at_source.remainder_wallet = Some(source.clone());
            let refused = at_source.init_withdraw_buffer(HashMap::from([(bundle.clone(), 1.0)]));
            assert!(matches!(refused, Err(KnishIOError::TransferRemainder)), "[{name}] remainder at S refused: {refused:?}");
            assert!(at_source.atoms.is_empty(), "[{name}] nothing built");
        }
        Ok(())
    }

    type StubResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    async fn read_json_body(socket: &mut TcpStream) -> StubResult<Value> {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&buffer[..end]).to_ascii_lowercase();
                let length: usize = head.lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                if buffer.len() >= end + 4 + length {
                    return Ok(serde_json::from_slice(&buffer[end + 4..end + 4 + length])?);
                }
            }
            let n = socket.read(&mut chunk).await?;
            if n == 0 {
                return Err("client closed the connection mid-request".into());
            }
            buffer.extend_from_slice(&chunk[..n]);
        }
    }

    async fn answer(mut socket: TcpStream, recorder: &tokio::sync::mpsc::UnboundedSender<Value>) -> StubResult<()> {
        let request = read_json_body(&mut socket).await?;
        let reply = if request["query"].as_str().unwrap_or_default().contains("ContinuId") {
            json!({ "data": { "ContinuId": null } })
        } else {
            json!({ "data": { "ProposeMolecule": {
                "molecularHash": "accepted-hash",
                "status": "accepted",
                "reason": null,
                "payload": "{\"token\":\"stub-jwt\",\"expiresAt\":4102444800,\"pubkey\":null}",
            } } })
        };
        recorder.send(request)?;
        let body = reply.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await?;
        socket.shutdown().await?;
        Ok(())
    }

    /// A loopback stand-in for the validator, so the public `create_token` runs with no network:
    /// it records every GraphQL body, answers ContinuId with no head (a fresh USER source) and
    /// accepts every proposed molecule, the login included.
    async fn loopback_validator() -> TestResult<(String, UnboundedReceiver<Value>)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}/graphql", listener.local_addr()?);
        let (recorder, received) = unbounded_channel();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                if answer(socket, &recorder).await.is_err() {
                    return;
                }
            }
        });
        Ok((url, received))
    }

    /// The public `create_token` with bare unit ids sends the C atom meta `tokenUnits` as the
    /// compact `[id, id, {}]` triples every other unit operation uses, byte for byte.
    #[tokio::test]
    async fn create_token_units_vectors() -> TestResult {
        let secret = generate_secret("create-token-units-vector");
        for test in section("create_token_units")? {
            let name = test["name"].as_str().unwrap_or_default();
            let (url, mut received) = loopback_validator().await?;
            let mut client = KnishIOClient::new(url.as_str(), Some("public".to_string()), None, None, None, Some(false));
            client.set_secret(secret.as_str());
            let meta = HashMap::from([("fungibility".to_string(), Value::from("stackable"))]);
            client.create_token(test["token"].as_str().ok_or("token")?, None, Some(meta), None, strings(&test["units"])?).await
                .map_err(|e| format!("[{name}] create_token: {e}"))?;

            let mut proposals = Vec::new();
            while let Ok(request) = received.try_recv() {
                if request["query"].as_str().unwrap_or_default().contains("ProposeMolecule") {
                    proposals.push(request["variables"]["molecule"].clone());
                }
            }
            let sent = proposals.last().ok_or("no molecule proposed")?;
            let c = &sent["atoms"][0];
            let wire_meta = |key: &str| c["meta"].as_array()?.iter().find(|m| m["key"] == key)?["value"].as_str();
            assert_eq!(c["isotope"], "C", "[{name}] the token creation C atom comes first");
            assert_eq!(wire_meta("tokenUnits"), test["expectedTokenUnits"].as_str(), "[{name}] tokenUnits, byte-exact");
            assert_eq!(c["value"].as_str(), test["expectedCValue"].as_str(), "[{name}] C value = unit count");
            assert_eq!(c["metaType"].as_str(), test["expectedMetaType"].as_str(), "[{name}] metaType");
            assert_eq!(c["metaId"].as_str(), test["expectedMetaId"].as_str(), "[{name}] metaId");

            let molecule = Molecule::from_json(sent, MoleculeFromJsonOptions::default())?;
            assert_eq!(molecule.atoms[0].isotope, Isotope::C, "[{name}] reconstructed C atom");
            assert_eq!(unit_ids(&molecule.atoms[0])?, strings(&test["expectedTokenUnitIds"])?, "[{name}] tokenUnits ids");
            molecule.check(None).map_err(|e| format!("[{name}] check: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(has_shared_fixtures)]
mod patent_vectors {
    use serde::Deserialize;
    use knishio_client::crypto::{
        shake256, generate_bundle_hash, generate_key, generate_address,
        generate_ots_signature, generate_secret,
        hex_to_base17, normalize_hash,
    };

    // ── JSON structures matching canonical-patent-vectors.json ───────────────

    #[derive(Deserialize)]
    struct PatentVectors {
        vectors: Vectors,
    }

    #[derive(Deserialize)]
    struct Vectors {
        generate_secret: Section<GenerateSecretTest>,
        continuid_chain: Section<ContinuIdTest>,
        base17_enumeration: Section<Base17Test>,
        multi_isotope_molecule: Section<MultiIsotopeTest>,
        bigint_carry_edge: Section<BigIntEdgeTest>,
        wots_roundtrip: Section<WotsRoundtripTest>,
        buffer_deposit_conservation: Section<BufferDepositTest>,
        buffer_withdraw_conservation: Section<BufferWithdrawTest>,
    }

    #[derive(Deserialize)]
    struct Section<T> {
        tests: Vec<T>,
    }

    // ── generateSecret (cross-SDK parity, Batch AO) ─────────────────────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GenerateSecretTest {
        name: String,
        seed: String,
        length: usize,
        expected_secret: String,
    }

    // ── ContinuID Chain ─────────────────────────────────────────────────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ContinuIdTest {
        name: String,
        secret: String,
        token: String,
        expected_bundle: String,
        position1: String,
        expected_address1: String,
        expected_position2: String,
        expected_address2: String,
    }

    // ── Base17 Enumeration ──────────────────────────────────────────────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Base17Test {
        name: String,
        hex_input: String,
        expected_base17: String,
        normalized_sum: i32,
    }

    // ── Multi-Isotope ───────────────────────────────────────────────────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MultiIsotopeTest {
        name: String,
        secret: String,
        expected_bundle: String,
        isotopes: std::collections::HashMap<String, IsotopeSpec>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct IsotopeSpec {
        expected_position: String,
        token: String,
        expected_address: String,
    }

    // ── BigInt Carry Edge ───────────────────────────────────────────────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BigIntEdgeTest {
        name: String,
        input: String,
        input_length: usize,
        expected_shake256: String,
        expected_base17_of_hash: String,
        expected_key_length: usize,
    }

    // ── WOTS+ Roundtrip ────────────────────────────────────────────────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WotsRoundtripTest {
        name: String,
        secret: String,
        token: String,
        position: String,
        expected_ots_address: String,
        molecular_hash_hex: String,
        molecular_hash_base17: String,
        expected_signature_fragment_count: usize,
        expected_signature_fragment0: String,
        expected_signature_fragment15: String,
        expected_verified: bool,
    }

    // ── Buffer deposit conservation (cross-SDK parity, Batch BF) ────────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BufferDepositTest {
        name: String,
        source_balance: i64,
        amount: f64,
        expected_source_value: String,
        expected_buffer_value: String,
        expected_remainder_value: String,
        expected_sum: String,
    }

    // ── Buffer withdraw conservation (cross-SDK parity, cycle 148) ──────────

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BufferWithdrawTest {
        name: String,
        source_balance: i64,
        amount: f64,
        expected_source_value: String,
        expected_recipient_value: String,
        expected_remainder_value: String,
        expected_sum: String,
    }

    // ── Load canonical vectors ──────────────────────────────────────────────

    const PATENT_VECTORS_JSON: &str = include_str!(
        "../../../sdks/shared-test-results/canonical-patent-vectors.json"
    );

    fn load_patent_vectors() -> PatentVectors {
        serde_json::from_str(PATENT_VECTORS_JSON)
            .expect("Failed to parse canonical-patent-vectors.json")
    }

    // ── Tests ───────────────────────────────────────────────────────────────

    /// Cross-SDK parity (Batch AO): generate_secret(seed) must produce the canonical
    /// 2048-hex secret, byte-identical to JS/TS/PHP/Python/Kotlin.
    #[test]
    fn test_generate_secret_vectors() {
        let vectors = load_patent_vectors();
        for test in &vectors.vectors.generate_secret.tests {
            let secret = generate_secret(&test.seed);
            assert_eq!(secret.len(), test.length,
                "generate_secret('{}') length mismatch", test.seed);
            assert_eq!(secret, test.expected_secret,
                "generate_secret('{}') value mismatch (cross-SDK parity)", test.seed);
        }
    }

    /// Cross-SDK parity (Batch BF): init_deposit_buffer must debit the FULL source
    /// balance so a partial buffer deposit still conserves (Σ V+B = 0), matching the
    /// JS/PHP/TS reference. (Pre-fix Rust debited only -amount → source = -amount and
    /// Σ = balance-amount ≠ 0, which the validator's b_isotope check rejects.)
    #[test]
    fn test_buffer_deposit_conservation_vectors() {
        use knishio_client::{Molecule, Wallet};
        use std::collections::HashMap;

        let vectors = load_patent_vectors();
        let secret = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let bundle = generate_bundle_hash(secret);
        let position = "1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b";

        for test in &vectors.vectors.buffer_deposit_conservation.tests {
            let mut source = Wallet::create(Some(secret), None, "BUFTOK", Some(position), None, None)
                .expect("create buffer source wallet");
            source.balance = test.source_balance.to_string();

            let mut mol = Molecule::with_params(
                Some(secret.to_string()),
                Some(bundle.clone()),
                Some(source),
                None, // remainder auto-derived from source (the change-routing path)
                Some("buftest".to_string()),
                None,
            );
            mol.init_deposit_buffer(test.amount, HashMap::new())
                .expect("init_deposit_buffer");

            // Inspect the wire form (isotope + value as strings), like BenchCtx does.
            let json = serde_json::to_value(&mol).expect("serialize buffer molecule");
            let atoms = json["atoms"].as_array().expect("atoms array");

            let mut sum: i128 = 0;
            let mut v_values: Vec<String> = Vec::new();
            let mut b_value: Option<String> = None;
            for a in atoms {
                let iso = a["isotope"].as_str().unwrap_or("");
                if iso == "V" || iso == "B" {
                    if let Some(v) = a["value"].as_str() {
                        sum += v.parse::<i128>()
                            .unwrap_or_else(|_| panic!("[{}] unparseable atom value '{}'", test.name, v));
                        if iso == "V" {
                            v_values.push(v.to_string());
                        } else {
                            b_value = Some(v.to_string());
                        }
                    }
                }
            }

            assert_eq!(sum.to_string(), test.expected_sum,
                "[{}] V+B conservation sum must be 0", test.name);
            // Emit order: source V (full-balance debit), buffer B (+amount), remainder V (+change).
            assert_eq!(v_values.first().map(String::as_str), Some(test.expected_source_value.as_str()),
                "[{}] source V-atom must debit the full balance", test.name);
            assert_eq!(b_value.as_deref(), Some(test.expected_buffer_value.as_str()),
                "[{}] buffer B-atom value", test.name);
            assert_eq!(v_values.get(1).map(String::as_str), Some(test.expected_remainder_value.as_str()),
                "[{}] remainder V-atom value", test.name);
        }
    }

    /// Cross-SDK parity (cycle 148): init_withdraw_buffer must debit the FULL source
    /// balance so a PARTIAL buffer withdraw still conserves (Σ V+B = 0), matching the
    /// JS/PHP/TS/Kotlin/C++ reference. (Pre-fix Rust debited only -total_amount → source =
    /// -amount and Σ = balance-amount ≠ 0 for a partial withdraw.) The withdraw analog of
    /// test_buffer_deposit_conservation_vectors. Atom order: source B (-balance), recipient
    /// V (+amount), remainder B (+(balance-amount)).
    #[test]
    fn test_buffer_withdraw_conservation_vectors() {
        use knishio_client::{Molecule, Wallet};
        use std::collections::HashMap;

        let vectors = load_patent_vectors();
        let secret = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let bundle = generate_bundle_hash(secret);
        let position = "1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b3c4d5e6f1a2b";

        for test in &vectors.vectors.buffer_withdraw_conservation.tests {
            let mut source = Wallet::create(Some(secret), None, "BUFTOK", Some(position), None, None)
                .expect("create buffer source wallet");
            source.balance = test.source_balance.to_string();

            let mut mol = Molecule::with_params(
                Some(secret.to_string()),
                Some(bundle.clone()),
                Some(source),
                None, // remainder auto-derived from source (the change-routing path)
                Some("buftest".to_string()),
                None,
            );
            // Withdraw `amount` to the caller's own bundle (mirrors the client wrapper).
            let mut recipients: HashMap<String, f64> = HashMap::new();
            recipients.insert(bundle.clone(), test.amount);
            mol.init_withdraw_buffer(recipients)
                .expect("init_withdraw_buffer");

            // Inspect the wire form (isotope + value as strings). Withdraw emits TWO B atoms
            // (source + remainder) and ONE V atom (recipient), unlike deposit's V-B-V.
            let json = serde_json::to_value(&mol).expect("serialize buffer molecule");
            let atoms = json["atoms"].as_array().expect("atoms array");

            let mut sum: i128 = 0;
            let mut b_values: Vec<String> = Vec::new();
            let mut v_values: Vec<String> = Vec::new();
            for a in atoms {
                let iso = a["isotope"].as_str().unwrap_or("");
                if iso == "V" || iso == "B" {
                    if let Some(v) = a["value"].as_str() {
                        sum += v.parse::<i128>()
                            .unwrap_or_else(|_| panic!("[{}] unparseable atom value '{}'", test.name, v));
                        if iso == "B" {
                            b_values.push(v.to_string());
                        } else {
                            v_values.push(v.to_string());
                        }
                    }
                }
            }

            assert_eq!(sum.to_string(), test.expected_sum,
                "[{}] V+B conservation sum must be 0", test.name);
            // Emit order: source B (full-balance debit), recipient V (+amount), remainder B (+change).
            assert_eq!(b_values.first().map(String::as_str), Some(test.expected_source_value.as_str()),
                "[{}] source B-atom must debit the full balance", test.name);
            assert_eq!(v_values.first().map(String::as_str), Some(test.expected_recipient_value.as_str()),
                "[{}] recipient V-atom value", test.name);
            assert_eq!(b_values.get(1).map(String::as_str), Some(test.expected_remainder_value.as_str()),
                "[{}] remainder B-atom value", test.name);
        }
    }

    /// Patent Claims 5, 12-14: ContinuID identity relay chain
    #[test]
    fn test_continuid_chain_vectors() {
        let vectors = load_patent_vectors();

        for test in &vectors.vectors.continuid_chain.tests {
            // Verify bundle hash
            let bundle = generate_bundle_hash(&test.secret);
            assert_eq!(bundle, test.expected_bundle,
                "Bundle hash mismatch for '{}'", test.name);

            // Verify wallet at position 1
            let key1 = generate_key(&test.secret, &test.token, &test.position1);
            let address1 = generate_address(&key1).unwrap();
            assert_eq!(address1, test.expected_address1,
                "Address1 mismatch for '{}'", test.name);

            // Verify ContinuID position derivation: position2 = SHAKE256(position1, 256)
            let position2 = shake256(&test.position1, 256);
            assert_eq!(position2, test.expected_position2,
                "Position2 derivation mismatch for '{}'", test.name);

            // Verify wallet at position 2
            let key2 = generate_key(&test.secret, &test.token, &position2);
            let address2 = generate_address(&key2).unwrap();
            assert_eq!(address2, test.expected_address2,
                "Address2 mismatch for '{}'", test.name);

            // Verify invariants
            assert_ne!(test.position1, position2, "Positions must differ");
            assert_ne!(address1, address2, "Addresses must differ");
        }
    }

    /// Patent Claim 5: Base17 encoding for WOTS+ OTS indexing
    #[test]
    fn test_base17_enumeration_vectors() {
        let vectors = load_patent_vectors();

        for test in &vectors.vectors.base17_enumeration.tests {
            let base17 = hex_to_base17(&test.hex_input).unwrap();
            assert_eq!(base17, test.expected_base17,
                "Base17 mismatch for '{}'", test.name);

            // Verify normalized sum = 0 (WOTS+ invariant)
            let normalized = normalize_hash(&base17);
            let sum: i32 = normalized.iter().map(|&x| x as i32).sum();
            assert_eq!(sum, test.normalized_sum,
                "Normalized sum mismatch for '{}': expected {}, got {}",
                test.name, test.normalized_sum, sum);
        }
    }

    /// Patent Claims 8, 21: Multi-isotope molecule composition
    #[test]
    fn test_multi_isotope_vectors() {
        let vectors = load_patent_vectors();

        for test in &vectors.vectors.multi_isotope_molecule.tests {
            let bundle = generate_bundle_hash(&test.secret);
            assert_eq!(bundle, test.expected_bundle,
                "Bundle mismatch for '{}'", test.name);

            let mut addresses = Vec::new();
            for (isotope_name, spec) in &test.isotopes {
                let key = generate_key(&test.secret, &spec.token, &spec.expected_position);
                let address = generate_address(&key).unwrap();
                assert_eq!(address, spec.expected_address,
                    "Address mismatch for '{}' isotope {} at position {}",
                    test.name, isotope_name, spec.expected_position);
                addresses.push(address);
            }

            // Verify all addresses are unique (different isotope positions → different wallets)
            let unique_count = {
                let mut sorted = addresses.clone();
                sorted.sort();
                sorted.dedup();
                sorted.len()
            };
            assert_eq!(unique_count, addresses.len(),
                "All isotope addresses must be unique for '{}'", test.name);
        }
    }

    /// Patent Claim 5: BigInt arithmetic edge cases
    #[test]
    fn test_bigint_carry_edge_vectors() {
        let vectors = load_patent_vectors();

        for test in &vectors.vectors.bigint_carry_edge.tests {
            // Verify input length
            assert_eq!(test.input.len(), test.input_length,
                "Input length mismatch for '{}'", test.name);

            // Verify SHAKE256 hash
            let hash = shake256(&test.input, 256);
            assert_eq!(hash, test.expected_shake256,
                "SHAKE256 mismatch for '{}'", test.name);
            assert_eq!(hash.len(), 64,
                "SHAKE256 output must be 64 hex chars for '{}'", test.name);

            // Verify Base17 of hash
            let base17 = hex_to_base17(&hash).unwrap();
            assert_eq!(base17, test.expected_base17_of_hash,
                "Base17(SHAKE256) mismatch for '{}'", test.name);

            // Verify key generation produces correct length
            let key = generate_key(
                &test.input,
                "USER",
                "0000000000000000000000000000000000000000000000000000000000000001",
            );
            assert_eq!(key.len(), test.expected_key_length,
                "Key length mismatch for '{}'", test.name);
        }
    }

    /// Patent Claims 1-2, 5: WOTS+ full sign/verify roundtrip
    #[test]
    fn test_wots_roundtrip_vectors() {
        let vectors = load_patent_vectors();

        for test in &vectors.vectors.wots_roundtrip.tests {
            let key = generate_key(&test.secret, &test.token, &test.position);

            // The OTS address is the two-pass protocol address (generate_address /
            // CheckMolecule::ots): hash each key chunk 16 times, join, then
            // digest = SHAKE256(joined, 8192) and address = SHAKE256(digest, 256).
            let ots_address = generate_address(&key).unwrap();
            assert_eq!(ots_address, test.expected_ots_address,
                "OTS address mismatch for '{}'", test.name);

            // Verify Base17 conversion of molecular hash
            let base17 = hex_to_base17(&test.molecular_hash_hex).unwrap();
            assert_eq!(base17, test.molecular_hash_base17,
                "Molecular hash Base17 mismatch for '{}'", test.name);

            // Generate signature
            let signature = generate_ots_signature(&key, &test.molecular_hash_base17).unwrap();
            assert_eq!(signature.len(), test.expected_signature_fragment_count,
                "Fragment count mismatch for '{}'", test.name);
            assert_eq!(signature[0], test.expected_signature_fragment0,
                "Fragment 0 mismatch for '{}'", test.name);
            assert_eq!(signature[15], test.expected_signature_fragment15,
                "Fragment 15 mismatch for '{}'", test.name);

            // Sign-then-verify roundtrip (two-pass, mirroring CheckMolecule::ots):
            // recover the public-key fragments from the signature (hash each fragment
            // 8 + normalized[i] times), join, then re-derive the address two-pass.
            let normalized = normalize_hash(&test.molecular_hash_base17);
            let mut recovered = String::new();
            for (i, fragment) in signature.iter().enumerate() {
                let mut working = fragment.clone();
                let iterations = (8 + normalized[i] as i32) as usize;
                for _ in 0..iterations {
                    working = shake256(&working, 512);
                }
                recovered.push_str(&working);
            }
            let recovered_digest = shake256(&recovered, 8192);
            let recovered_address = shake256(&recovered_digest, 256);
            assert_eq!(recovered_address, test.expected_ots_address,
                "Roundtrip recovered address mismatch for '{}'", test.name);
            let verified = recovered_address == test.expected_ots_address;
            assert_eq!(verified, test.expected_verified,
                "Verification mismatch for '{}': expected {}", test.name, test.expected_verified);
        }
    }

    /// The frozen Phase B fixture mirror (tests/fixtures/mod.rs) must equal the master sections,
    /// so the standalone Phase B vector tests exercise exactly the canonical vectors.
    #[test]
    fn phase_b_fixture_mirror_matches_master() -> Result<(), serde_json::Error> {
        let master: serde_json::Value = serde_json::from_str(PATENT_VECTORS_JSON)?;
        let mirror: serde_json::Value = serde_json::from_str(super::fixtures::PHASE_B_VECTORS_JSON)?;
        let sections = ["token_replenish", "stackable_fusion_conservation", "buffer_withdraw_fresh_remainder", "create_token_units"];
        assert_eq!(mirror.as_object().map(|m| m.len()), Some(sections.len()), "the mirror holds exactly the Phase B sections");
        for section in sections {
            assert_eq!(mirror[section], master["vectors"][section], "fixture section {section} drifted from the master");
        }
        Ok(())
    }
}
