use secp256k1::{PublicKey, SecretKey};
use rand::{Rng, rngs::OsRng};
use bitcoin::network::constants::Network;
use bitcoin::util::key::PrivateKey;
use bitcoin::Address;

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
    let (private_key, address) = create_keypair();

    // Print the generated key pair
    println!("Private key: {}", private_key.to_wif());
    println!("Address: {}", address);
}