//! Frozen fixtures: secret-storage envelopes and the Phase B vector sections.
//!
//! Shared single-source-of-truth between `tests/secret_storage.rs` (which decrypts it standalone)
//! and `tests/cross_platform_vectors.rs` (which verifies parity with master vector tests[0] and tests[1] when fixtures are present).
//! Pinned by cross-platform drift guards — do not regenerate.
pub const FROZEN_TS_ENVELOPE: &str = r#"{"version":1,"ciphertext":"dQjZ4cR+ZBefuF4xSib8Qv/H2oZ5Qv8mRCRmQuLaCDYoBaRMqPQRullxZsID","iv":"3Q8ArAH0ZEgYFpoE","salt":"4LWNzAFGrY4SzcPulKMVcg==","algorithm":"AES-GCM","iterations":100000,"metadata":{"bundleHash":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef","label":"probe","createdAt":1788558526546,"hardwareBacked":false,"providerType":"webcrypto-aes-gcm"}}"#;

pub const FROZEN_LEGACY_0_9_5_ENVELOPE: &str = r#"{"version":1,"ciphertext":"jf1+FJAP8itTOuBH8gur0lnmkGYps1TQyx/Q+3Ah+kAk8eC9/gt2DjZ5tz4v","iv":"XjR5FNNDRipgM6hx","salt":"lhpi9ywH55D6ZnUIrnr8+Q==","algorithm":"AES-GCM","iterations":100000,"metadata":{"bundle_hash":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef","label":"probe","created_at":1788558634482,"hardware_backed":false,"provider_type":"aes-gcm"}}"#;

pub const XSDK_BUNDLE: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
pub const XSDK_PASSPHRASE: &str = "cross-sdk-pass";
pub const XSDK_PLAINTEXT: &str = "MASTER-SECRET-CROSS-SDK-PROBE";

pub const FROZEN_JS_1_1_0_RECOVERY_ENVELOPE: &str = r#"{"version":1,"ciphertext":"L5aLK9OsXSQKxUDZYuKp6XRdqgMvmuXo6aphBnzDW7B/DLD0ydx0SByRFJaGyedWvakM","iv":"e1qXQy3KppMCPCfv","salt":"wcTJpsmFGXaMRyyQOcg/Qw==","algorithm":"AES-GCM","iterations":100000,"metadata":{"bundleHash":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef","createdAt":1700000000000,"hardwareBacked":false,"providerType":"webcrypto-aes-gcm","label":"backup-recovery"}}"#;

pub const XSDK_RECOVERY_PASSPHRASE: &str = "recovery-passphrase-restore-42";
pub const XSDK_RECOVERY_PLAINTEXT: &str = "RECOVERY-SECRET-PROBE-MATRIX-BACKUP";
pub const XSDK_REENROLLED_PRIMARY_PASSPHRASE: &str = "xsdk-reenrolled-primary-pass";

/// Frozen copy of the Phase B vector sections of the master
/// `sdks/shared-test-results/canonical-patent-vectors.json` (`token_replenish`,
/// `stackable_fusion_conservation`, `buffer_withdraw_fresh_remainder`), so the Phase B vector tests
/// in `tests/patent_vector_validation.rs` run in a standalone checkout. Its gated suite asserts that
/// this copy equals the master sections — do not edit by hand.
#[allow(dead_code)] // read only by tests/patent_vector_validation.rs; the other suites share this file
pub const PHASE_B_VECTORS_JSON: &str = r#"{
  "token_replenish": {
    "description": "replenishToken mints additional supply of an EXISTING token with a C atom (the minting isotope), never with V atoms. Emitted atoms (in order): C (signed by the identity's USER wallet exactly like the SDK's own createToken C atom; metaType 'token', metaId = token slug, value = +amount for fungible or the new unit count for stackable/non-fungible; metas in order: action = 'add', then address/position/pubkey of the credited wallet (the identity's existing wallet for the token, or a new one), batchId only when that wallet has one, tokenUnits only for stackable/non-fungible), then the SDK's standard I ContinuID remainder atom. Validator 0.6.0 accepts it only from the token's creator bundle, for supply 'infinite' or 'replenishable', crediting the creator's own bundle. Replaces the historical 2-V replenish (V(+amount), V(+(balance+amount))) that JS/TS/PHP/Rust built and every CheckMolecule rejects. Cross-SDK parity lock (Phase B, validator 0.6.0 / SDK patch wave, 2026-09-26).",
    "tests": [
      {
        "name": "fungible_replenish",
        "token": "REPLTOK",
        "amount": 500,
        "units": [],
        "expectedIsotopes": [
          "C",
          "I"
        ],
        "expectedCValue": "500",
        "expectedMetaType": "token",
        "expectedMetaId": "REPLTOK",
        "expectedAction": "add",
        "expectedTokenUnitIds": null
      },
      {
        "name": "stackable_replenish",
        "token": "REPLSTK",
        "amount": null,
        "units": [
          [
            "R1",
            "R1",
            {}
          ],
          [
            "R2",
            "R2",
            {}
          ]
        ],
        "expectedIsotopes": [
          "C",
          "I"
        ],
        "expectedCValue": "2",
        "expectedMetaType": "token",
        "expectedMetaId": "REPLSTK",
        "expectedAction": "add",
        "expectedTokenUnitIds": [
          "R1",
          "R2"
        ]
      }
    ]
  },
  "stackable_fusion_conservation": {
    "description": "fuseToken fuses M >= 2 units of a stackable token held by the source wallet (balance B = unit count, B >= M) into ONE new unit N delivered to the recipient bundle. Emitted atoms (in order): source V = -B (tokenUnits = the M fused units in the source order: the SENT set; the kept units appear only on the remainder atom); burn V = +(M-1) (metaType walletBundle, metaId = 64 zeros, tokenUnits = the fused ids except the LAST, caller order); F = +1 (metaType walletBundle, metaId = recipient bundle, tokenUnits = [N] where N.metas.fusedTokenUnits = the full triples of all M fused units, caller order); remainder V = +(B-M) (metaType walletBundle, metaId = sender bundle, tokenUnits = the source units minus the fused ones, source order; emitted even when B = M with value 0 and tokenUnits []). No I (ContinuID) atom: like transferToken and burnToken, the molecule is signed by the source token wallet at its own position, and the validator requires any I-bearing molecule to sign at the ContinuID pointer. The sum of all V+F values MUST equal 0; the last fused unit is absorbed by the F atom (validator 0.6.0 tombstones it and inserts N). Source units are U1..U5 (triples [id, id, {}]) unless the test lists sourceUnits; N id is 'FUSED'. Cross-SDK parity lock (Phase B, validator 0.6.0 / SDK patch wave, 2026-09-26).",
    "tests": [
      {
        "name": "fuse_2_of_2",
        "sourceUnits": [
          "U1",
          "U2"
        ],
        "fuse": [
          "U1",
          "U2"
        ],
        "newUnitId": "FUSED",
        "expectedIsotopes": [
          "V",
          "V",
          "F",
          "V"
        ],
        "expectedSourceValue": "-2",
        "expectedSourceUnitIds": [
          "U1",
          "U2"
        ],
        "expectedBurnValue": "1",
        "expectedFusionValue": "1",
        "expectedRemainderValue": "0",
        "expectedBurnUnitIds": [
          "U1"
        ],
        "expectedRemainderUnitIds": [],
        "expectedFusedTokenUnitIds": [
          "U1",
          "U2"
        ],
        "expectedSum": "0"
      },
      {
        "name": "fuse_2_of_5",
        "sourceUnits": [
          "U1",
          "U2",
          "U3",
          "U4",
          "U5"
        ],
        "fuse": [
          "U2",
          "U4"
        ],
        "newUnitId": "FUSED",
        "expectedIsotopes": [
          "V",
          "V",
          "F",
          "V"
        ],
        "expectedSourceValue": "-5",
        "expectedSourceUnitIds": [
          "U2",
          "U4"
        ],
        "expectedBurnValue": "1",
        "expectedFusionValue": "1",
        "expectedRemainderValue": "3",
        "expectedBurnUnitIds": [
          "U2"
        ],
        "expectedRemainderUnitIds": [
          "U1",
          "U3",
          "U5"
        ],
        "expectedFusedTokenUnitIds": [
          "U2",
          "U4"
        ],
        "expectedSum": "0"
      },
      {
        "name": "fuse_3_of_5",
        "sourceUnits": [
          "U1",
          "U2",
          "U3",
          "U4",
          "U5"
        ],
        "fuse": [
          "U2",
          "U4",
          "U5"
        ],
        "newUnitId": "FUSED",
        "expectedIsotopes": [
          "V",
          "V",
          "F",
          "V"
        ],
        "expectedSourceValue": "-5",
        "expectedSourceUnitIds": [
          "U2",
          "U4",
          "U5"
        ],
        "expectedBurnValue": "2",
        "expectedFusionValue": "1",
        "expectedRemainderValue": "2",
        "expectedBurnUnitIds": [
          "U2",
          "U4"
        ],
        "expectedRemainderUnitIds": [
          "U1",
          "U3"
        ],
        "expectedFusedTokenUnitIds": [
          "U2",
          "U4",
          "U5"
        ],
        "expectedSum": "0"
      },
      {
        "name": "fuse_single_unit_rejected",
        "sourceUnits": [
          "U1",
          "U2",
          "U3",
          "U4",
          "U5"
        ],
        "fuse": [
          "U1"
        ],
        "newUnitId": "FUSED",
        "mustReject": true,
        "expectedErrorContains": "at least two token units"
      }
    ]
  },
  "buffer_withdraw_fresh_remainder": {
    "description": "init_withdraw_buffer after validator 0.6.1 / SDK patch wave (contract 9.6): the source B atom is the actual buffer wallet S (Balance(type: buffer)), value -S.balance; one addressless recipient V = +amount per recipient (metaType walletBundle, metaId recipient bundle); the remainder B = +(S.balance-amount) goes to S.createRemainder(secret), a FRESH position that MUST differ from S's position (a remainder at the signing position is credited behind a consumed one-time key and is stranded; validator 0.6.1 rejects it with 'Value may not be credited to a consumed signing position'). No I atom. Emitted even when the remainder value is 0. Sum of V+B values MUST be 0.",
    "tests": [
      {
        "name": "partial_withdraw_fresh_remainder",
        "sourceBalance": 50,
        "amount": 20,
        "expectedIsotopes": [
          "B",
          "V",
          "B"
        ],
        "expectedSourceValue": "-50",
        "expectedRecipientValue": "20",
        "expectedRemainderValue": "30",
        "expectedSum": "0",
        "expectedRemainderPositionDistinctFromSource": true
      },
      {
        "name": "full_withdraw_fresh_remainder",
        "sourceBalance": 50,
        "amount": 50,
        "expectedIsotopes": [
          "B",
          "V",
          "B"
        ],
        "expectedSourceValue": "-50",
        "expectedRecipientValue": "50",
        "expectedRemainderValue": "0",
        "expectedSum": "0",
        "expectedRemainderPositionDistinctFromSource": true
      }
    ]
  }
}"#;
