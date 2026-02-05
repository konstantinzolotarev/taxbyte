# Security: PDF Generation from URL

## ✅ Current Security Status: PROTECTED

The invoice PDF generation endpoint is now **IP whitelisted** (localhost only):

- **Endpoint**: `GET /invoices/{id}/html`
- **Authentication**: IP Whitelist (127.0.0.1, ::1)
- **Protection**: Only localhost can access
- **Used by**: wkhtmltopdf for PDF generation

## ✅ Implemented Security: IP Whitelist

**Status**: Implemented (Option 2)

The endpoint now validates the peer IP address and only allows:
- `127.0.0.1` (IPv4 localhost)
- `::1` (IPv6 localhost)

Any request from a non-localhost IP is rejected with `403 Forbidden`.

**Implementation**: `src/adapters/http/handlers/invoices_web.rs:579-605`
```rust
let peer_addr = req.peer_addr()?;
let is_localhost = peer_addr.ip().is_loopback();

if !is_localhost {
  return Err(ApiError::Auth(AuthErrorKind::Forbidden));
}
```

## 🔍 Why This Exists

This enables URL-based PDF generation:
1. User marks invoice as "Sent"
2. Backend generates PDF using wkhtmltopdf
3. wkhtmltopdf fetches HTML from `http://127.0.0.1:8080/invoices/{id}/html`
4. PDF is created and optionally uploaded to cloud storage

## 🛡️ Required Security Fixes

### Option 1: Time-Limited Access Tokens (Recommended)

Generate a short-lived JWT token when starting PDF generation:

```rust
// Generate token (valid for 30 seconds)
let token = generate_pdf_token(invoice_id, expires_in_secs: 30);
let url = format!("{}/invoices/{}/html?token={}", base_url, invoice_id, token);

// In handler: validate token
if !validate_pdf_token(token, invoice_id) {
  return Err(ApiError::Unauthorized);
}
```

### Option 2: IP Whitelist (Simple)

Only allow localhost to access the endpoint:

```rust
// In middleware or handler
let peer_addr = req.peer_addr();
if !is_localhost(peer_addr) {
  return Err(ApiError::Forbidden);
}
```

### Option 3: Internal Service Port

Run PDF generation on a separate internal port (e.g., 8081):

```toml
[server]
public_port = 8080
internal_port = 8081  # Only accessible from localhost
```

### Option 4: Revert to File-Based Generation

Go back to generating HTML files (original approach):
- More disk I/O
- Less elegant
- But more secure (no public endpoint)

## 📋 Implementation Status

- [x] Choose security approach (Option 2: IP Whitelist)
- [x] Implement IP whitelist in handler
- [x] Update `invoice_html_view` handler
- [x] Update security comments in invoice service
- [x] Update documentation
- [ ] Add rate limiting per invoice ID (optional enhancement)
- [ ] Add integration tests for IP whitelist (recommended)
- [ ] Consider Option 1 (JWT tokens) for distributed deployments

## 🔧 Affected Code Locations

### 1. Service Layer
**File**: `src/domain/invoice/services.rs:430-435`
```rust
// TODO: SECURITY - Remove this bypass after implementing token auth
if !user_id.is_nil() {
  self.verify_company_membership(user_id, invoice.company_id).await?;
}
```

### 2. Handler
**File**: `src/adapters/http/handlers/invoices_web.rs:579-609`
```rust
// TODO: SECURITY WARNING - This endpoint is PUBLIC
pub async fn invoice_html_view(...)
```

### 3. Routes
**File**: `src/adapters/http/routes.rs:234-240`
```rust
// TODO: SECURITY - Public invoice HTML view
cfg.service(web::resource("/invoices/{id}/html")...)
```

## 🎯 Production Readiness

**✅ Safe for Production** (with caveats):
- IP whitelist implemented (localhost only)
- Suitable for single-server deployments
- wkhtmltopdf must run on the same machine
- For distributed/cloud deployments, consider Option 1 (JWT tokens)

## 📝 Notes

- For development/testing: Current approach is acceptable
- For production: MUST implement one of the security fixes above
- Invoice UUIDs provide some security-by-obscurity (not ideal)
- Consider adding audit logging for all invoice HTML access
