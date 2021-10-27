use client_rust::Client;

fn main() {
    println!("Let's say hello to a Solana account...");

    // Establish a connection to the cluster
    let mut client = Client::new();
    println!("connection established, version: {}", client.get_version().unwrap());

    // Determine who pays for fees
    client.establish_payer();

    // Check if the program has been deployed
    let key = client.check_program();

    // Say hello to an account
    client.say_hello(&key);

    // Find out how many times the account has been greeted
    client.report(&key);

    println!("Success");
}
