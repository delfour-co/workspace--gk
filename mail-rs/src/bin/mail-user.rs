//! CLI tool for managing SMTP users
//!
//! This tool provides commands to manage user accounts for SMTP authentication.
//!
//! # Usage
//!
//! ```bash
//! # Add a new user
//! mail-user add user@example.com password123 --db sqlite://users.db
//!
//! # Delete a user
//! mail-user delete user@example.com --db sqlite://users.db
//!
//! # List all users
//! mail-user list --db sqlite://users.db
//!
//! # Check if user exists
//! mail-user exists user@example.com --db sqlite://users.db
//! ```

use clap::{Parser, Subcommand};
use mail_rs::security::Authenticator;

#[derive(Parser)]
#[command(name = "mail-user")]
#[command(about = "Manage SMTP user accounts", long_about = None)]
struct Cli {
    /// Database URL (e.g., sqlite://users.db)
    #[arg(short, long, default_value = "sqlite://users.db")]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new user
    Add {
        /// User email address
        email: String,
        /// User password
        password: String,
    },
    /// Delete a user
    Delete {
        /// User email address
        email: String,
    },
    /// List all users
    List,
    /// Check if user exists
    Exists {
        /// User email address
        email: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize authenticator
    let auth = Authenticator::new(&cli.db).await?;

    match cli.command {
        Commands::Add { email, password } => {
            println!("Adding user: {}", email);

            // Check if user already exists
            if auth.user_exists(&email).await? {
                eprintln!("Error: User {} already exists", email);
                std::process::exit(1);
            }

            auth.add_user(&email, &password).await?;
            println!("✓ User {} added successfully", email);
        }
        Commands::Delete { email } => {
            println!("Deleting user: {}", email);

            // Check if user exists
            if !auth.user_exists(&email).await? {
                eprintln!("Error: User {} does not exist", email);
                std::process::exit(1);
            }

            auth.delete_user(&email).await?;
            println!("✓ User {} deleted successfully", email);
        }
        Commands::List => {
            println!("Listing all users...\n");

            let users = auth.list_users_detailed().await?;

            if users.is_empty() {
                println!("No users found.");
            } else {
                println!("{:<30} {:<20} {:<20}", "Email", "Created At", "Last Login");
                println!("{:-<70}", "");

                for (email, created_at, last_login) in &users {
                    let last_login_str = last_login.as_deref().unwrap_or("Never");
                    println!("{:<30} {:<20} {:<20}", email, created_at, last_login_str);
                }

                println!("\nTotal: {} user(s)", users.len());
            }
        }
        Commands::Exists { email } => {
            if auth.user_exists(&email).await? {
                println!("✓ User {} exists", email);
            } else {
                println!("✗ User {} does not exist", email);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
