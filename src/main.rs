use secp256k1::{PublicKey, SecretKey};
use rand::{Rng, rngs::OsRng};
use bitcoin::network::constants::Network;
use bitcoin::util::key::PrivateKey;
use bitcoin::Address;
use std::thread;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, BufRead};
use std::path::Path;
use std::time::Instant;

const BASE58_CHARS: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const PRACTICE_RUNS: u32 = 10;

fn create_keypair() -> (PrivateKey, Address) {
    let secp = secp256k1::Secp256k1::new();
    let mut rng = OsRng;
    let secret_key = SecretKey::from_slice(&mut rng.gen::<[u8; 32]>()).expect("Unable to generate secret key");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    let public_key = bitcoin::PublicKey {
        compressed: true,
        key: public_key,
    };

    let private_key = PrivateKey {
        compressed: true,
        network: Network::Bitcoin,
        key: secret_key,
    };
    let address = Address::p2pkh(&public_key, Network::Bitcoin);

    (private_key, address)
}

fn read_target_substrings(file_path: &str) -> Result<Vec<String>, io::Error> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);

    let mut substrings = Vec::new();
    for line in reader.lines() {
        substrings.push(line?);
    }

    Ok(substrings)
}

fn is_base58(s: &str) -> bool {
    s.chars().all(|c| BASE58_CHARS.contains(c))
}

fn main() {
    let max_threads = 2;
    let output_file = "keypairs.txt";
    let echo_interval: u64 = 100000;

    // let target_substrings = ["brodude", "SwEeT", "BU"];

    let target_substrings_file = "target_substrings.txt";
    let target_substrings = match read_target_substrings(target_substrings_file) {
        Ok(substrings) => substrings,
        Err(err) => {
            eprintln!("Error reading target substrings file: {}", err);
            return;
        }
    };

    for s in &target_substrings {
        if !is_base58(s) {
            eprintln!("Error: '{}' contains non-Base58 characters.", s);
            return;
        }
    }

    let (tx, rx) = mpsc::channel();

    // Run practice rounds to measure the performance
    let mut durations = Vec::new();
    for _ in 0..PRACTICE_RUNS {
        let start_time = Instant::now();
        let (_private_key, _address) = create_keypair();
        //@todo - include matching attempt:
        //let _ = check_for_matching_substrings(&address, &target_substrings);
        durations.push(start_time.elapsed().as_micros());
    }

    // Calculate the median duration
    durations.sort();
    let median_duration = durations[durations.len() / 2];

    // Calculate the probability of finding a matching address

    let address_length = 34;
    let base58_length = 58;

    // Find the longest substring in the list
    let longest_substring = target_substrings.iter().max_by_key(|s| s.len()).unwrap();
    let longest_substring_length = longest_substring.len();

    if longest_substring_length == address_length {
        println!("Don't be daft. Trying to crack a BTC address is basically impossible and trying is a terrible waste of resources.");
        return;
    }

    // Find the shortest substring in the list
    let shortest_substring = target_substrings.iter().min_by_key(|s| s.len()).unwrap();
    let target_length = shortest_substring.len();

    let probability = 1.0 - (1.0 - (1.0 / base58_length as f64).powi(target_length as i32)).powi((address_length - target_length + 1) as i32);
    let attempts_needed = (1.0 / probability).ceil() as u64;
    let estimated_time = (attempts_needed as u128 * median_duration) / 1_000_000;

    println!(
        "Estimated time to find a matching address: {} seconds",
        estimated_time
    );

    println!("Are you sure you want to proceed? (y/n)");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    if input.trim().to_lowercase() != "y" {
        return;
    }

    let search_start_time = Instant::now();

    let found = Arc::new(AtomicBool::new(false));
    let finished_threads = Arc::new(AtomicUsize::new(0));

    for _ in 0..max_threads {
        let thread_tx = tx.clone();
        let target_substrings = target_substrings.clone();
        let found = found.clone();
        let finished_threads = finished_threads.clone();

        thread::spawn(move || {
            let mut attempts: u64 = 0;

            loop {
                if found.load(Ordering::SeqCst) {
                    break;
                }

                let (private_key, address) = create_keypair();
                let address_str = address.to_string();
                attempts += 1;

                if target_substrings.iter().any(|substr| address_str.contains(substr)) {
                    thread_tx.send((private_key, address)).unwrap();
                    found.store(true, Ordering::SeqCst);
                    break;
                }

                if attempts % echo_interval == 0 {
                    println!("Still nothing found after {} attempts...", attempts);
                }

            }

            finished_threads.fetch_add(1, Ordering::SeqCst);
        });
    }

    loop {
        if finished_threads.load(Ordering::SeqCst) >= max_threads {
            break;
        }
    }

    match rx.try_recv() {
        Ok((private_key, address)) => {

            let elapsed_time = search_start_time.elapsed().as_secs();

            println!("Found private key: {}", private_key.to_wif());
            println!("Found address: {}", address);
            println!("Time taken to find the matching address: {} seconds", elapsed_time);

            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(output_file)
                .expect("Unable to open output file");

            writeln!(file, "Private key: {}", private_key.to_wif()).expect("Unable to write private key to file");
            writeln!(file, "Address: {}\n", address).expect("Unable to write address to file");
        }
        Err(_) => {
            println!("No matching address found.");
        }
    }
}
