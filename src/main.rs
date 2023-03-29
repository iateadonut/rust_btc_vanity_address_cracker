use secp256k1::{PublicKey, SecretKey};
use rand::{Rng, rngs::OsRng};
use bitcoin::network::constants::Network;
use bitcoin::util::key::PrivateKey;
use bitcoin::Address;
use std::thread;
use std::sync::mpsc;

fn create_keypair() -> (PrivateKey, Address) {
    // Generate a random secret key
    let secp = secp256k1::Secp256k1::new();
    let mut rng = OsRng;
    let secret_key = SecretKey::from_slice(&mut rng.gen::<[u8; 32]>()).expect("Unable to generate secret key");

    // Calculate the public key
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // Convert the secp256k1 public key to a bitcoin public key
    let public_key = bitcoin::PublicKey {
        compressed: true,
        key: public_key,
    };

    // Generate the Bitcoin private key and address
    let private_key = PrivateKey {
        compressed: true,
        network: Network::Bitcoin,
        key: secret_key,
    };
    let address = Address::p2pkh(&public_key, Network::Bitcoin);

    (private_key, address)
}

fn main() {
    let num_threads = 100;
    let (tx, rx) = mpsc::channel();

    for _ in 0..num_threads {
        let thread_tx = tx.clone();
        thread::spawn(move || {
            let keypair = create_keypair();
            thread_tx.send(keypair).unwrap();
        });
    }

    for _ in 0..num_threads {
        let (private_key, address) = rx.recv().unwrap();
        println!("Private key: {}", private_key.to_wif());
        println!("Address: {}", address);
    }
}