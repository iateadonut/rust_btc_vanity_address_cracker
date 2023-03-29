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

fn main() {
    let max_threads = 2;
    let output_file = "keypairs.txt";

    // let target_substrings = ["brodude", "SwEeT", "BU"];

    let target_substrings_file = "target_substrings.txt";
    let target_substrings = match read_target_substrings(target_substrings_file) {
        Ok(substrings) => substrings,
        Err(err) => {
            eprintln!("Error reading target substrings file: {}", err);
            return;
        }
    };

    let (tx, rx) = mpsc::channel();
    let found = Arc::new(AtomicBool::new(false));
    let finished_threads = Arc::new(AtomicUsize::new(0));

    for _ in 0..max_threads {
        let thread_tx = tx.clone();
        let target_substrings = target_substrings.clone();
        let found = found.clone();
        let finished_threads = finished_threads.clone();

        thread::spawn(move || {
            loop {
                if found.load(Ordering::SeqCst) {
                    break;
                }

                let (private_key, address) = create_keypair();
                let address_str = address.to_string();

                if target_substrings.iter().any(|substr| address_str.contains(substr)) {
                    thread_tx.send((private_key, address)).unwrap();
                    found.store(true, Ordering::SeqCst);
                    break;
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
            println!("Found private key: {}", private_key.to_wif());
            println!("Found address: {}", address);

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
