# mail-user CLI Tool

Command-line tool for managing SMTP user accounts for authentication.

## Installation

```bash
cargo build --release --bin mail-user
# Binary will be at: target/release/mail-user
```

## Usage

### Add a new user

```bash
mail-user --db sqlite://users.db add user@example.com SecurePassword123
```

**Output:**
```
Adding user: user@example.com
✓ User user@example.com added successfully
```

### List all users

```bash
mail-user --db sqlite://users.db list
```

**Output:**
```
Listing all users...

Email                          Created At           Last Login
----------------------------------------------------------------------
admin@example.com              2024-01-15 10:30:00  2024-01-20 14:25:33
user@example.com               2024-01-10 09:15:22  Never

Total: 2 user(s)
```

### Check if user exists

```bash
mail-user --db sqlite://users.db exists user@example.com
```

**Output:**
```
✓ User user@example.com exists
```

Exit code: 0 if exists, 1 if not

### Delete a user

```bash
mail-user --db sqlite://users.db delete user@example.com
```

**Output:**
```
Deleting user: user@example.com
✓ User user@example.com deleted successfully
```

## Configuration

The `--db` parameter specifies the database URL. It defaults to `sqlite://users.db`.

**Examples:**
```bash
# Use default database (users.db in current directory)
mail-user list

# Use custom database path
mail-user --db sqlite:///var/mail/users.db list

# Use absolute path
mail-user --db sqlite:///etc/mail-rs/users.db add admin@domain.com password
```

## Security Notes

- Passwords are hashed using **Argon2** before storage
- The database file should have restricted permissions (e.g., `chmod 600 users.db`)
- Consider using environment variables or password prompts for production use (to avoid passwords in shell history)

## Integration with mail-rs

To use authentication in the SMTP server, configure it in `config.toml`:

```toml
[smtp]
enable_auth = true
auth_database_url = "sqlite://users.db"
require_auth = true  # Optional: require auth for sending
```

Then start the server:

```bash
cargo run --release
```
