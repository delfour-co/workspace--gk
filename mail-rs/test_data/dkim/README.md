# DKIM Test Keys

This directory contains DKIM RSA key pair for testing purposes.

**⚠️ WARNING: These are TEST KEYS ONLY. DO NOT use in production!**

## Files

- `dkim_private.pem` - Private RSA key (2048-bit) for signing emails
- `dkim_public.pem` - Public RSA key for verification

## Usage

### For Signing (Outbound Emails)

```rust
use mail_rs::authentication::dkim::DkimSigner;
use std::path::Path;

let signer = DkimSigner::new(
    "example.com".to_string(),
    "default".to_string(),
    Path::new("test_data/dkim/dkim_private.pem")
)?;

let message = b"From: test@example.com\r\nTo: recipient@example.com\r\n\r\nBody";
let signed_message = signer.sign_and_prepend(message)?;
```

### DNS TXT Record

For DKIM to work, you need to publish the public key in DNS:

**Record Name**: `default._domainkey.example.com`
**Type**: TXT
**Value**: Extract from `dkim_public.pem` and format as:

```
v=DKIM1; k=rsa; p=<base64_public_key_without_headers>
```

#### Extract Public Key for DNS:

```bash
# Remove headers and newlines from public key
cat dkim_public.pem | grep -v "BEGIN PUBLIC KEY" | grep -v "END PUBLIC KEY" | tr -d '\n'
```

Example DNS TXT record:
```
v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA1234567890...
```

## Configuration in mail-rs

Add to `config.toml`:

```toml
[dkim]
enabled = true
domain = "example.com"
selector = "default"
private_key_path = "test_data/dkim/dkim_private.pem"
sign_outbound = true  # Sign all outbound emails
```

## Generating New Keys

If you need to generate new test keys:

```bash
# Generate private key
openssl genrsa -out dkim_private.pem 2048

# Extract public key
openssl rsa -in dkim_private.pem -pubout -out dkim_public.pem
```

For production, use longer keys (3072 or 4096 bits) and store securely:

```bash
# Production key (4096-bit)
openssl genrsa -out dkim_private_prod.pem 4096
chmod 600 dkim_private_prod.pem  # Secure permissions
```

## Security Notes

1. **Private key** must be kept secure - only readable by mail server
2. **Rotate keys** every 6-12 months
3. Use different keys for different selectors (e.g., `mail`, `auto`, `bulk`)
4. Monitor DKIM validation failures in logs

## Testing DKIM

### Test Signing

```bash
cargo test --package mail-rs dkim_signer
```

### Test with External Validators

Send test email to:
- `check-auth@verifier.port25.com` - Returns DKIM/SPF report
- `check-auth2@verifier.port25.com` - Alternative validator

### Online Tools

- https://dkimvalidator.com/ - Check DKIM signatures
- https://mxtoolbox.com/dkim.aspx - DNS record validator
