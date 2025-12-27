# Chimney HTTPS Test Setup

This directory contains a complete test configuration for Chimney's HTTPS support with manual certificates.

## 📁 Directory Structure

```
test-https/
├── chimney.toml              # Main server configuration
├── localhost-cert.pem        # Self-signed certificate for localhost
├── localhost-key.pem         # Private key for localhost
├── sites/
│   └── localhost/
│       ├── chimney.toml      # Site-specific HTTPS config
│       └── public/
│           └── index.html    # Test page
└── README.md                 # This file
```

## 🚀 Quick Start

### 1. Run the server

From the repository root:

```bash
# Using cargo run
cargo run -- --config test-https/chimney.toml

# Or if you built the release binary
./target/release/chimney --config test-https/chimney.toml
```

**Note:** You'll need sudo/admin privileges to bind to port 443:

```bash
# On Linux/macOS
sudo cargo run -- --config test-https/chimney.toml

# Or allow binding to privileged ports
sudo setcap CAP_NET_BIND_SERVICE=+eip ./target/release/chimney
./target/release/chimney --config test-https/chimney.toml
```

### 2. Access the test page

- **HTTPS (secure):** https://localhost
- **HTTP (redirects):** http://localhost:8080

**Browser Security Warning:** You'll see a security warning because we're using a self-signed certificate. This is expected for local testing.

- **Chrome/Edge:** Click "Advanced" → "Proceed to localhost (unsafe)"
- **Firefox:** Click "Advanced" → "Accept the Risk and Continue"
- **Safari:** Click "Show Details" → "visit this website"

## 🧪 What Gets Tested

✅ **Manual Certificate Loading**
   - Loading certificate and private key from PEM files
   - SNI (Server Name Indication) for multiple domains
   - TLS 1.2+ with secure cipher suites

✅ **Dual Listeners**
   - HTTP listener on port 8080
   - HTTPS listener on port 443
   - Independent connection handling

✅ **HTTP → HTTPS Redirect**
   - Automatic 301 redirects from HTTP to HTTPS
   - Preserves URL path and query parameters

✅ **Security Features**
   - Path traversal protection (site names, cert paths)
   - Private key permissions (0600 on Unix)
   - Certificate validation

## 📝 Configuration Details

### Main Config (`chimney.toml`)

```toml
host = "0.0.0.0"
port = 8080              # HTTP listener
sites_directory = "sites"
log_level = "debug"
```

### Site Config (`sites/localhost/chimney.toml`)

```toml
root = "public"
domain_names = ["localhost", "127.0.0.1"]

[https_config]
enabled = true
auto_issue = false       # Manual mode (not ACME)
auto_redirect = true     # HTTP → HTTPS redirect

cert_file = "../../localhost-cert.pem"
key_file = "../../localhost-key.pem"
```

## 🔐 Certificate Details

The self-signed certificates were generated with:

```bash
openssl req -x509 -newkey rsa:2048 \
  -keyout localhost-key.pem \
  -out localhost-cert.pem \
  -days 365 -nodes \
  -subj "/CN=localhost"
```

**Properties:**
- **Subject:** CN=localhost
- **Valid for:** 365 days from generation
- **Key type:** RSA 2048-bit
- **Usage:** Testing only (not for production)

## 🔄 Testing Different Scenarios

### Test HTTP Redirect

```bash
# Should redirect to https://localhost
curl -I http://localhost:8080
# Expect: HTTP/1.1 301 Moved Permanently
#         Location: https://localhost/
```

### Test HTTPS Connection

```bash
# Access via HTTPS (accept self-signed cert)
curl -k https://localhost

# Or verify certificate details
openssl s_client -connect localhost:443 -servername localhost
```

### Test Multiple Domains

The site responds to both `localhost` and `127.0.0.1`. Try:

```bash
curl -k https://localhost
curl -k https://127.0.0.1
```

### View Server Logs

The server runs with `log_level = "debug"` to see:
- TLS initialization
- Certificate loading
- Connection handling
- Redirect decisions

## 🎯 Next Steps

After verifying manual certificates work:

1. **Test with real domain + ACME**
   - Change `auto_issue = true`
   - Add `acme_email = "your@email.com"`
   - Point domain to your server
   - Let's Encrypt will issue real certificates

2. **Test wildcard certificates**
   - Generate cert for `*.example.local`
   - Add multiple subdomains to `domain_names`
   - Verify SNI routing works

3. **Test certificate renewal**
   - Replace certificates while server is running
   - Verify graceful reload (future feature)

## 🐛 Troubleshooting

### "Permission denied" on port 443

Port 443 requires root/admin privileges:

```bash
sudo cargo run -- --config test-https/chimney.toml
```

Or grant capabilities (Linux):

```bash
sudo setcap CAP_NET_BIND_SERVICE=+eip ./target/release/chimney
```

### "Address already in use"

Another service is using port 443 or 8080:

```bash
# Check what's using the ports
sudo lsof -i :443
sudo lsof -i :8080

# Stop conflicting services or change ports in chimney.toml
```

### Browser shows "Connection refused"

Make sure the server is running and listening:

```bash
# Check if ports are listening
netstat -tuln | grep -E '(443|8080)'
```

### Certificate not found errors

Verify paths in `sites/localhost/chimney.toml` are correct:

```bash
ls -la test-https/localhost-*.pem
```

## 📚 Resources

- **Chimney Documentation:** ../README.md
- **TLS Module:** ../crates/chimney-core/src/tls/
- **ACME Setup Guide:** (coming soon)
- **Let's Encrypt Docs:** https://letsencrypt.org/docs/
