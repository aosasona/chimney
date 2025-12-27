# Testing ACME/Let's Encrypt on a Public Server

This guide explains how to test automatic HTTPS certificate issuance using ACME (Let's Encrypt) with Chimney on a public server.

## Prerequisites

### 1. Domain Requirements
- A **public domain name** pointing to your server's IP address
- DNS A record must be configured (e.g., `example.com` → `203.0.113.10`)
- Domain must be publicly accessible (not localhost)

### 2. Server Requirements
- **Port 80 (HTTP)** must be open and accessible from the internet
  - Required for ACME HTTP-01 challenge validation
  - Let's Encrypt will make HTTP requests to verify domain ownership

- **Port 443 (HTTPS)** must be open and accessible from the internet
  - Required for serving HTTPS traffic

- **Root/sudo access** to bind to privileged ports (80, 443)

### 3. Network Configuration
```bash
# Check if ports are open (run on server)
sudo netstat -tlnp | grep ':80\|:443'

# Test external accessibility (run from another machine)
curl -I http://your-domain.com
```

## Configuration

### Option 1: Let's Encrypt Staging (Recommended for Testing)

Create `acme-test-staging.toml`:

```toml
# Main server configuration
host = "0.0.0.0"
port = 80  # HTTP listener for ACME challenges
log_level = "debug"

sites_directory = "sites"

# Optional: Custom cache directory for certificates
# cache_directory = "/var/cache/chimney/certs"
```

Create `sites/example/chimney.toml`:

```toml
root = "public"
domain_names = ["example.com", "www.example.com"]

[https_config]
enabled = true
auto_issue = true  # Enable ACME
auto_redirect = true  # Redirect HTTP → HTTPS

# REQUIRED: Email for Let's Encrypt notifications
acme_email = "admin@example.com"

# Use staging for testing (no rate limits)
acme_directory = "https://acme-staging-v02.api.letsencrypt.org/directory"
```

### Option 2: Let's Encrypt Production (After Testing)

**⚠️ WARNING: Production has rate limits**
- 50 certificates per domain per week
- 5 failed validations per hour
- Use staging first!

```toml
[https_config]
enabled = true
auto_issue = true
auto_redirect = true
acme_email = "admin@example.com"

# Production (default, can be omitted)
acme_directory = "https://acme-v02.api.letsencrypt.org/directory"
```

## Running the Test

### Step 1: Set Up Directory Structure

```bash
# On your public server
mkdir -p sites/example/public
echo "ACME Test Success!" > sites/example/public/index.html
```

### Step 2: Configure Domain

Update `sites/example/chimney.toml` with your actual domain:
```toml
domain_names = ["yourdomain.com", "www.yourdomain.com"]
acme_email = "you@yourdomain.com"
```

### Step 3: Run with Sudo

```bash
# Must run as root to bind to ports 80 and 443
sudo chimney-cli serve --config acme-test-staging.toml
```

### Step 4: Monitor Certificate Issuance

Watch the logs for ACME events:
```
[INFO] Initializing ACME for site 'example' with domains: ["yourdomain.com", "www.yourdomain.com"]
[INFO] ACME event for site 'example': CertCached
[INFO] ACME manager initialized for site 'example' with 2 domain(s)
[INFO] TLS is enabled, starting dual listeners (HTTP + HTTPS)
[INFO] HTTP server listening on 0.0.0.0:80
[INFO] HTTPS server listening on 0.0.0.0:443
```

### Step 5: Test HTTPS Connection

```bash
# From another machine or browser
curl -I https://yourdomain.com

# Check certificate details
openssl s_client -connect yourdomain.com:443 -servername yourdomain.com < /dev/null 2>/dev/null | openssl x509 -noout -text
```

## Certificate Storage

Certificates are automatically cached in:
- Default: `<config-dir>/.chimney/certs/<site-name>/`
- Custom: `<cache_directory>/<site-name>/`

Example:
```
.chimney/certs/example/
├── cert.pem       # Certificate chain
└── key.pem        # Private key (permissions: 0600)
```

## ACME Challenge Flow

1. **Client connects** via HTTPS to `https://yourdomain.com`
2. **TLS handshake begins**, SNI hostname sent
3. **ACME manager checks** for cached certificate
4. **If no certificate**:
   - ACME initiates HTTP-01 challenge
   - Let's Encrypt requests: `http://yourdomain.com/.well-known/acme-challenge/<token>`
   - Chimney responds with challenge proof
   - Let's Encrypt validates and issues certificate
   - Certificate is cached to disk
5. **TLS handshake completes** with new certificate

## Automatic Renewal

Certificates are automatically renewed when:
- Certificate expires in < 30 days
- Background task checks every 24 hours
- New certificate is issued and swapped atomically

## Troubleshooting

### Port 80/443 Already in Use
```bash
# Find process using ports
sudo lsof -i :80
sudo lsof -i :443

# Stop conflicting service
sudo systemctl stop nginx  # or apache2
```

### DNS Not Propagating
```bash
# Check DNS resolution
dig yourdomain.com +short
nslookup yourdomain.com

# Wait for propagation (can take up to 48 hours)
```

### ACME Challenge Failing
```bash
# Test if challenge endpoint is accessible
curl http://yourdomain.com/.well-known/acme-challenge/test

# Check firewall rules
sudo iptables -L -n | grep -E '80|443'
sudo ufw status
```

### Rate Limit Hit (Production Only)
```
Error: too many certificates already issued for exact set of domains
```
**Solution**: Wait 7 days or use staging environment

### Certificate Not Trusted (Staging)
This is **expected** for staging certificates. Staging uses a fake CA for testing.

**Production certificates** are trusted by all major browsers.

## Example: Complete Test Setup

```bash
# 1. Create directory structure
mkdir -p acme-test/sites/mysite/public

# 2. Create main config
cat > acme-test/acme.toml <<EOF
host = "0.0.0.0"
port = 80
log_level = "debug"
sites_directory = "sites"
EOF

# 3. Create site config
cat > acme-test/sites/mysite/chimney.toml <<EOF
root = "public"
domain_names = ["mysite.example.com"]

[https_config]
enabled = true
auto_issue = true
auto_redirect = true
acme_email = "admin@example.com"
acme_directory = "https://acme-staging-v02.api.letsencrypt.org/directory"
EOF

# 4. Create test page
cat > acme-test/sites/mysite/public/index.html <<EOF
<!DOCTYPE html>
<html>
<head><title>ACME Test</title></head>
<body>
<h1>HTTPS with ACME Works! 🎉</h1>
<p>This page is served over HTTPS with a Let's Encrypt certificate.</p>
</body>
</html>
EOF

# 5. Run server (requires sudo)
cd acme-test
sudo chimney-cli serve --config acme.toml

# 6. Test from browser
# Visit: https://mysite.example.com
```

## Security Notes

1. **Never commit certificates** - `.chimney/certs/` should be in `.gitignore`
2. **Protect private keys** - Automatically set to `0600` permissions on Unix
3. **Use production** only after staging tests pass
4. **Monitor renewal** - Check logs for renewal failures
5. **Keep email current** - Let's Encrypt sends expiration warnings to `acme_email`

## Multi-Domain Setup

You can host multiple sites with different certificates:

```toml
# sites/site1/chimney.toml
domain_names = ["site1.com", "www.site1.com"]
[https_config]
enabled = true
auto_issue = true
acme_email = "admin@site1.com"

# sites/site2/chimney.toml
domain_names = ["site2.com", "www.site2.com"]
[https_config]
enabled = true
auto_issue = true
acme_email = "admin@site2.com"
```

Each site gets its own certificate via SNI (Server Name Indication).

## Wildcard Certificates

**Note**: HTTP-01 challenge doesn't support wildcards. For wildcard certificates:
- Use DNS-01 challenge (not yet implemented in Chimney)
- Or use manual certificates: `*.example.com`

## Monitoring Certificate Status

```bash
# Check certificate expiration
openssl s_client -connect yourdomain.com:443 -servername yourdomain.com < /dev/null 2>/dev/null \
  | openssl x509 -noout -dates

# Expected output:
# notBefore=Dec 27 00:00:00 2025 GMT
# notAfter=Mar 27 00:00:00 2026 GMT
```

## Let's Encrypt Rate Limits

**Staging**: No rate limits (for testing)

**Production**:
- 50 certificates per registered domain per week
- 300 pending authorizations per account
- 5 duplicate certificates per week
- 5 failed validation attempts per hour

Full details: https://letsencrypt.org/docs/rate-limits/
