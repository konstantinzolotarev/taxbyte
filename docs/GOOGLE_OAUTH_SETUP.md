# Google OAuth 2.0 Setup Guide for TaxByte

This guide explains how to set up Google OAuth 2.0 credentials for the TaxByte Google Drive integration.

## Prerequisites

- A Google account
- Access to [Google Cloud Console](https://console.cloud.google.com/)
- Admin or Owner access to a Google Cloud Project (or ability to create one)

---

## Step 1: Create a Google Cloud Project

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Click the project dropdown at the top of the page
3. Click **"New Project"**
4. Enter project details:
   - **Project name**: `taxbyte` (or any name you prefer)
   - **Organization**: Select your organization (or leave as "No organization")
5. Click **"Create"**
6. Wait for the project to be created (a notification will appear)
7. Select your new project from the project dropdown

---

## Step 2: Enable the Google Drive API

1. In the Google Cloud Console, navigate to **"APIs & Services"** > **"Library"**
   - Or use direct link: https://console.cloud.google.com/apis/library
2. Search for **"Google Drive API"**
3. Click on **"Google Drive API"** from the search results
4. Click the **"Enable"** button
5. Wait for the API to be enabled

---

## Step 3: Configure the OAuth Consent Screen

1. Navigate to **"APIs & Services"** > **"OAuth consent screen"**
   - Or use direct link: https://console.cloud.google.com/apis/credentials/consent
2. Select user type:
   - **External**: For testing with any Google account
   - **Internal**: Only if using Google Workspace (limits to your organization)
3. Click **"Create"**

### Fill in App Information:

**App information:**
- **App name**: `TaxByte`
- **User support email**: Your email address
- **App logo**: (Optional) Upload your app logo

**App domain:**
- **Application home page**: `http://localhost:8080` (for development)
- **Privacy policy link**: (Optional for development)
- **Terms of service link**: (Optional for development)

**Developer contact information:**
- **Email addresses**: Your email address

4. Click **"Save and Continue"**

### Add Scopes:

5. Click **"Add or Remove Scopes"**
6. Find and select:
   - `https://www.googleapis.com/auth/drive.file` (View and manage Google Drive files created by this app)
7. Click **"Update"**
8. Click **"Save and Continue"**

### Add Test Users (for External apps):

9. Click **"Add Users"**
10. Enter email addresses of users who will test the app
11. Click **"Add"**
12. Click **"Save and Continue"**

### Review:

13. Review your settings
14. Click **"Back to Dashboard"**

---

## Step 4: Create OAuth 2.0 Credentials

1. Navigate to **"APIs & Services"** > **"Credentials"**
   - Or use direct link: https://console.cloud.google.com/apis/credentials
2. Click **"+ Create Credentials"** at the top
3. Select **"OAuth client ID"**

### Configure OAuth Client:

4. **Application type**: Select **"Web application"**
5. **Name**: `TaxByte Web Client` (or any descriptive name)

### Authorized JavaScript origins:

6. Click **"+ Add URI"**
7. Add your application URLs:
   - For development: `http://localhost:8080`
   - For production: `https://yourdomain.com`

### Authorized redirect URIs:

8. Click **"+ Add URI"**
9. Add your OAuth callback URLs:
   - For development: `http://localhost:8080/oauth/google/callback`
   - For production: `https://yourdomain.com/oauth/google/callback`

10. Click **"Create"**

### Save Your Credentials:

11. A dialog will appear with your **Client ID** and **Client Secret**
12. **IMPORTANT**: Copy both values immediately:
    - **Client ID**: looks like `123456789012-abcdefghijklmnop.apps.googleusercontent.com`
    - **Client secret**: looks like `GOCSPX-abcdefghijklmnopqrstuvwxyz`
13. Click **"OK"**

---

## Step 5: Configure TaxByte Environment Variables

Add the following to your `.env` file:

```bash
# Google OAuth 2.0 credentials
GOOGLE_OAUTH_CLIENT_ID=your-client-id.apps.googleusercontent.com
GOOGLE_OAUTH_CLIENT_SECRET=GOCSPX-your-client-secret

# OAuth redirect URL (must match Google Cloud Console)
GOOGLE_OAUTH_REDIRECT_URL=http://localhost:8080/oauth/google/callback

# Token encryption key (generate with: openssl rand -base64 32)
ENCRYPTION_KEY_BASE64=your-base64-encryption-key
```

### Generate Encryption Key:

```bash
# On macOS/Linux:
openssl rand -base64 32

# Copy the output and paste it as ENCRYPTION_KEY_BASE64
```

---

## Step 6: Verify Setup

### Start the Application:

```bash
cargo run
```

### Test OAuth Flow:

1. Navigate to `http://localhost:8080`
2. Log in to TaxByte
3. Go to **Company Settings** > **Storage** tab
4. Click **"Connect Google Drive"**
5. You should be redirected to Google's OAuth consent screen
6. Review the permissions requested
7. Click **"Allow"** or **"Continue"**
8. You should be redirected back to TaxByte
9. The storage settings should show "Google Drive Connected"

### Test Connection:

10. Click the **"Test Connection"** button
11. You should see a success message if everything is configured correctly

---

## Development Mode (Mock OAuth)

For development without Google OAuth credentials, you can use mock OAuth:

### Enable Mock OAuth:

Add to your `.env` file:

```bash
MOCK_OAUTH=true
```

### Using Mock OAuth:

1. Start the application
2. Click "Connect Google Drive"
3. You'll see a mock consent screen instead of Google's
4. Click "Grant Access"
5. OAuth flow completes with fake tokens

**Note**: Mock OAuth doesn't actually connect to Google Drive. Invoice uploads will fail, but the OAuth flow UI/UX can be tested.

---

## Production Deployment

### Update OAuth Consent Screen:

1. Go back to **"OAuth consent screen"** in Google Cloud Console
2. Click **"Publish App"** to make it available to all users
3. You may need to submit for Google verification if using sensitive scopes

### Update Redirect URIs:

1. Go to **"Credentials"** in Google Cloud Console
2. Click on your OAuth 2.0 Client ID
3. Under **"Authorized redirect URIs"**, add your production URL:
   - `https://yourdomain.com/oauth/google/callback`
4. Click **"Save"**

### Update Environment Variables:

```bash
# Production OAuth redirect URL
GOOGLE_OAUTH_REDIRECT_URL=https://yourdomain.com/oauth/google/callback
```

---

## Troubleshooting

### Error: "redirect_uri_mismatch"

**Cause**: The redirect URI used by the app doesn't match any authorized redirect URIs in Google Cloud Console.

**Solution**:
1. Check the exact URL in the error message
2. Add that exact URL to **"Authorized redirect URIs"** in Google Cloud Console
3. Make sure there are no trailing slashes or URL differences

### Error: "Access blocked: This app's request is invalid"

**Cause**: OAuth consent screen is not configured or app is not published.

**Solution**:
1. Complete the OAuth consent screen configuration (Step 3)
2. Add your email as a test user
3. Or publish the app for public use

### Error: "invalid_client"

**Cause**: Client ID or Client Secret is incorrect or doesn't match.

**Solution**:
1. Verify `GOOGLE_OAUTH_CLIENT_ID` and `GOOGLE_OAUTH_CLIENT_SECRET` in `.env`
2. Make sure you copied the entire value including any dashes or special characters
3. Regenerate credentials if necessary

### Error: "Token encryption failed"

**Cause**: `ENCRYPTION_KEY_BASE64` is not set or is invalid.

**Solution**:
1. Generate a new key: `openssl rand -base64 32`
2. Add it to `.env` as `ENCRYPTION_KEY_BASE64`
3. Restart the application

---

## Security Best Practices

### Protect Your Credentials:

- ✅ **Never** commit `.env` to version control
- ✅ Use environment variables for production
- ✅ Rotate client secrets periodically
- ✅ Use separate OAuth clients for development and production

### Token Storage:

- ✅ Tokens are encrypted at rest using AES-256-GCM
- ✅ Encryption key must be stored securely (environment variable or secrets manager)
- ✅ Never log or expose refresh tokens

### OAuth Scope:

- ✅ TaxByte uses `drive.file` scope (restricted access)
- ✅ Only accesses files created by the app
- ✅ Cannot read or modify other files in user's Drive

---

## Additional Resources

- [Google OAuth 2.0 Documentation](https://developers.google.com/identity/protocols/oauth2)
- [Google Drive API Overview](https://developers.google.com/drive/api/guides/about-sdk)
- [OAuth 2.0 Scopes for Google APIs](https://developers.google.com/identity/protocols/oauth2/scopes#drive)

---

## Support

If you encounter issues not covered in this guide:

1. Check the application logs for detailed error messages
2. Verify all environment variables are set correctly
3. Ensure Google Drive API is enabled in your project
4. Review Google Cloud Console audit logs for failed auth attempts

For development questions, check the project documentation or create an issue on GitHub.
