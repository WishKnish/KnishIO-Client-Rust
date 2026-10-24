//! Batch verification throughput: sequential vs a persistent rayon pool of T threads.
//!
//!   cargo bench --features parallel[,kcore] --bench parallel_verify -- --label <name>
//!
//! Workloads: M copies of a served OTS v1 or OTS v2 molecule (tests/fixtures, compiled in, so
//! the built binary runs stand-alone on an appliance), verified with
//! `CheckMolecule::verify_from_server_data`. Every (ots, M, T) cell is first checked against the
//! sequential result; timing then runs 5 rounds with the cells interleaved inside each round,
//! each measurement repeating its call for at least 0.5 s. ops = molecules; medians over rounds.

use knishio_client::CheckMolecule;
use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde_json::Value;
use std::error::Error;
use std::hint::black_box;
use std::time::{Duration, Instant};

const MOLECULES: [usize; 5] = [1, 4, 16, 64, 256];
const ROUNDS: usize = 5;
const MIN_TIMED: Duration = Duration::from_millis(500);

type Verdict = (bool, Option<String>, Option<String>);

fn sequential(refs: &[&Value]) -> Vec<Verdict> {
    refs.iter().map(|m| verdict(m)).collect()
}

fn pooled(pool: &ThreadPool, refs: &[&Value]) -> Vec<Verdict> {
    pool.install(|| refs.par_iter().map(|m| verdict(m)).collect())
}

fn verdict(molecule: &Value) -> Verdict {
    let r = CheckMolecule::verify_from_server_data(molecule);
    (r.verified, r.error, r.molecular_hash)
}

/// Molecules per second over at least MIN_TIMED of back-to-back calls.
fn rate(molecules: usize, mut call: impl FnMut() -> Vec<Verdict>) -> f64 {
    let start = Instant::now();
    let mut calls = 0usize;
    while start.elapsed() < MIN_TIMED {
        black_box(call());
        calls += 1;
    }
    (calls * molecules) as f64 / start.elapsed().as_secs_f64()
}

fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let label = args
        .iter()
        .position(|a| a == "--label")
        .and_then(|i| args.get(i + 1))
        .map_or("host", String::as_str)
        .to_string();

    let fixture: Value = serde_json::from_str(include_str!("../tests/fixtures/served-molecules.json"))?;
    let sets = [("otsv1", &fixture["v1"]), ("otsv2", &fixture["v2"])];

    let nproc = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut threads = vec![1, 2, 4, nproc];
    threads.sort_unstable();
    threads.dedup();
    let pools = threads
        .iter()
        .map(|&t| ThreadPoolBuilder::new().num_threads(t).build().map(|p| (t, p)))
        .collect::<Result<Vec<_>, _>>()?;

    let backend = if knishio_client::kcore::available() { "kcore" } else { "pure" };
    println!(
        "PARBENCH-ENV label={label} backend={backend} nproc={nproc} utc={}",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );

    for (ots, molecule) in sets {
        for m in MOLECULES {
            let refs = vec![molecule; m];
            let expected = sequential(&refs);
            let all_verified = expected.iter().all(|v| v.0);
            let identical = pools.iter().all(|(_, pool)| pooled(pool, &refs) == expected);
            if !all_verified || !identical {
                println!("PARBENCH-CHECK identical=no ots={ots} M={m} verified={all_verified} parallel_equal={identical}");
                std::process::exit(1);
            }
        }
    }
    println!("PARBENCH-CHECK identical=yes");

    // samples[set][m][0] = sequential, samples[set][m][1 + t] = pool t
    let mut samples = vec![vec![vec![Vec::with_capacity(ROUNDS); 1 + pools.len()]; MOLECULES.len()]; sets.len()];
    for _ in 0..ROUNDS {
        for (s, (_, molecule)) in sets.iter().enumerate() {
            for (mi, &m) in MOLECULES.iter().enumerate() {
                let refs = vec![*molecule; m];
                samples[s][mi][0].push(rate(m, || sequential(&refs)));
                for (ti, (_, pool)) in pools.iter().enumerate() {
                    samples[s][mi][1 + ti].push(rate(m, || pooled(pool, &refs)));
                }
            }
        }
    }

    for (s, (ots, _)) in sets.iter().enumerate() {
        for (mi, &m) in MOLECULES.iter().enumerate() {
            let seq = median(&samples[s][mi][0]);
            for (ti, (t, _)) in pools.iter().enumerate() {
                let par = median(&samples[s][mi][1 + ti]);
                println!(
                    "PARBENCH label={label} backend={backend} ots={ots} M={m} T={t} seq_ops={seq:.0} par_ops={par:.0} ratio={:.3}",
                    par / seq
                );
            }
        }
    }
    Ok(())
}
