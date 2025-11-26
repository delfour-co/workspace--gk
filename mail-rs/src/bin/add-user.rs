//! Add a user to the authentication database

use mail_rs::security::Authenticator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <email> <password>", args[0]);
        eprintln!("Example: {} test@example.com password123", args[0]);
        std::process::exit(1);
    }

    let email = &args[1];
    let password = &args[2];

    println!("Adding user: {}", email);

    let auth = Authenticator::new("sqlite://mail.db").await?;
    auth.add_user(email, password).await?;

    println!("✅ User added successfully");
    println!("   Email: {}", email);
    println!("   Password: {}", password);

    Ok(())
}
