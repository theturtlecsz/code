//! HTTP callback server for OAuth 2.0 authorization code receipt.
//!
//! Provides a simple HTTP server that listens for OAuth callbacks,
//! validates the state parameter, and extracts the authorization code.
//!
//! # Example
//!
//! ```rust,ignore
//! use codex_core::provider_auth::callback_server::CallbackServer;
//! use std::time::Duration;
//!
//! let server = CallbackServer::new()?;
//! let port = server.port();
//! // Use port in redirect_uri...
//!
//! // Wait for callback
//! let code = server.wait_for_code("expected-state", Duration::from_secs(300))?;
//! ```

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

use super::AuthError;

/// Simple HTTP server to receive OAuth callbacks.
pub struct CallbackServer {
    listener: TcpListener,
    port: u16,
}

impl CallbackServer {
    /// Creates a new callback server on an available port.
    ///
    /// # Returns
    ///
    /// A new `CallbackServer` bound to localhost on a randomly assigned port.
    ///
    /// # Errors
    ///
    /// Returns an error if binding to localhost fails.
    pub fn new() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(false)?;
        Ok(Self { listener, port })
    }

    /// Creates a callback server on a specific port.
    ///
    /// Useful when the OAuth provider requires a fixed redirect URI.
    ///
    /// # Arguments
    ///
    /// * `port` - The port number to bind to
    ///
    /// # Errors
    ///
    /// Returns an error if the port is already in use.
    pub fn on_port(port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
        listener.set_nonblocking(false)?;
        Ok(Self { listener, port })
    }

    /// Returns the port the server is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Waits for an OAuth callback and returns the authorization code.
    ///
    /// Blocks until a callback is received or the timeout expires.
    /// Validates the state parameter for CSRF protection.
    ///
    /// # Arguments
    ///
    /// * `expected_state` - The state parameter sent in the authorization request
    /// * `timeout` - Maximum time to wait for the callback
    ///
    /// # Returns
    ///
    /// The authorization code from the OAuth callback.
    ///
    /// # Errors
    ///
    /// - `AuthError::Io` if the connection fails
    /// - `AuthError::OAuth` if state doesn't match or code is missing
    /// - `AuthError::CallbackTimeout` if timeout expires
    pub fn wait_for_code(
        &self,
        expected_state: &str,
        timeout: Duration,
    ) -> Result<String, AuthError> {
        // Set listener to non-blocking for timeout handling
        self.listener.set_nonblocking(true).map_err(AuthError::Io)?;

        // Poll for connection with timeout
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        let (mut stream, _) = loop {
            match self.listener.accept() {
                Ok(conn) => break conn,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= timeout {
                        return Err(AuthError::CallbackTimeout);
                    }
                    std::thread::sleep(poll_interval);
                }
                Err(e) => return Err(AuthError::Io(e)),
            }
        };

        // Set stream to blocking with timeout for reading
        stream.set_nonblocking(false).map_err(AuthError::Io)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(AuthError::Io)?;

        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .map_err(AuthError::Io)?;

        // Parse the callback parameters
        let (code, state, error) = parse_callback_params(&request_line)?;

        // Check for OAuth error response
        if let Some(error_code) = error {
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n\
                <!DOCTYPE html><html><head><title>Authentication Failed</title></head>\
                <body><h1>Authentication failed</h1><p>Error: {error_code}</p></body></html>"
            );
            let _ = stream.write_all(response.as_bytes());
            return Err(AuthError::OAuth {
                error: error_code,
                description: "OAuth authorization was denied or failed".to_string(),
            });
        }

        // Validate state for CSRF protection
        if state != expected_state {
            let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n\
                <!DOCTYPE html><html><head><title>Security Error</title></head>\
                <body><h1>Security error</h1><p>State parameter mismatch. \
                Please try authenticating again.</p></body></html>";
            let _ = stream.write_all(response.as_bytes());
            return Err(AuthError::OAuth {
                error: "state_mismatch".to_string(),
                description: "CSRF protection: state parameter mismatch".to_string(),
            });
        }

        // Ensure we have a code
        let code = code.ok_or_else(|| AuthError::OAuth {
            error: "missing_code".to_string(),
            description: "No authorization code in callback".to_string(),
        })?;

        // Send success response
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: #f5f5f5;
        }
        .container {
            text-align: center;
            padding: 40px;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1 {
            color: #10b981;
            margin-bottom: 16px;
        }
        p {
            color: #666;
            margin-bottom: 24px;
        }
        .checkmark {
            font-size: 48px;
            margin-bottom: 16px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="checkmark">✓</div>
        <h1>Authentication Successful!</h1>
        <p>You can close this window and return to the terminal.</p>
    </div>
    <script>
        // Try to close the window after a short delay
        setTimeout(function() {
            window.close();
        }, 2000);
    </script>
</body>
</html>"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            html.len(),
            html
        );
        stream
            .write_all(response.as_bytes())
            .map_err(AuthError::Io)?;
        stream.flush().map_err(AuthError::Io)?;

        Ok(code)
    }
}

/// Parses callback parameters from an HTTP request line.
///
/// Extracts `code`, `state`, and `error` from the query string.
///
/// # Arguments
///
/// * `request_line` - The HTTP request line (e.g., "GET /?code=XXX&state=YYY HTTP/1.1")
///
/// # Returns
///
/// A tuple of (Option<code>, state, Option<error>).
fn parse_callback_params(
    request_line: &str,
) -> Result<(Option<String>, String, Option<String>), AuthError> {
    // Parse "GET /?code=XXX&state=YYY HTTP/1.1"
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| AuthError::InvalidResponse("Missing path in callback".to_string()))?;

    let query = path.split('?').nth(1).ok_or_else(|| {
        AuthError::InvalidResponse("Missing query string in callback".to_string())
    })?;

    let mut code = None;
    let mut state = String::new();
    let mut error = None;

    for param in query.split('&') {
        let mut parts = param.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");

        // URL decode the value
        let decoded = urlencoding_decode(value);

        match key {
            "code" => code = Some(decoded),
            "state" => state = decoded,
            "error" => error = Some(decoded),
            _ => {}
        }
    }

    if state.is_empty() && error.is_none() {
        return Err(AuthError::InvalidResponse(
            "Missing state parameter in callback".to_string(),
        ));
    }

    Ok((code, state, error))
}

/// Simple URL decoding for OAuth parameters.
///
/// Handles `%XX` encoding and `+` as space.
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            }
            '+' => result.push(' '),
            _ => result.push(c),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_server_creation() {
        let server = CallbackServer::new();
        assert!(server.is_ok());
        let server = server.unwrap();
        assert!(server.port() > 0);
    }

    #[test]
    fn test_parse_callback_params_success() {
        let request = "GET /?code=abc123&state=xyz789 HTTP/1.1";
        let (code, state, error) = parse_callback_params(request).unwrap();
        assert_eq!(code, Some("abc123".to_string()));
        assert_eq!(state, "xyz789");
        assert!(error.is_none());
    }

    #[test]
    fn test_parse_callback_params_with_error() {
        let request = "GET /?error=access_denied&state=xyz789 HTTP/1.1";
        let (code, state, error) = parse_callback_params(request).unwrap();
        assert!(code.is_none());
        assert_eq!(state, "xyz789");
        assert_eq!(error, Some("access_denied".to_string()));
    }

    #[test]
    fn test_parse_callback_params_url_encoded() {
        let request = "GET /?code=abc%2B123&state=xyz%3D789 HTTP/1.1";
        let (code, state, error) = parse_callback_params(request).unwrap();
        assert_eq!(code, Some("abc+123".to_string()));
        assert_eq!(state, "xyz=789");
        assert!(error.is_none());
    }

    #[test]
    fn test_parse_callback_params_missing_query() {
        let request = "GET / HTTP/1.1";
        let result = parse_callback_params(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_callback_params_missing_state() {
        let request = "GET /?code=abc123 HTTP/1.1";
        let result = parse_callback_params(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_urlencoding_decode() {
        assert_eq!(urlencoding_decode("hello%20world"), "hello world");
        assert_eq!(urlencoding_decode("hello+world"), "hello world");
        assert_eq!(urlencoding_decode("a%2Bb"), "a+b");
        assert_eq!(urlencoding_decode("no%encoding"), "no%encoding"); // Invalid hex
    }
}
