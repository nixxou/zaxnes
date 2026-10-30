//! The requests to the RetroAchievements server, made on threads of their own.

use std::time::Duration;

/// The status rcheevos reads as "no answer, try again later"
/// (`RC_API_SERVER_RESPONSE_RETRYABLE_CLIENT_ERROR`).
pub const RETRYABLE_ERROR: i32 = -2;

/// The largest answer read: a game's achievement set is a few hundred kilobytes at most.
const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;

/// A request of the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// The address, with its query.
    pub url: String,
    /// The form posted, or `None` for a GET.
    pub post_data: Option<String>,
    /// The type of the form posted.
    pub content_type: Option<String>,
    /// The `User-Agent` the server wants: the program and the rcheevos version.
    pub user_agent: String,
}

/// The answer to a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// The HTTP status, or [`RETRYABLE_ERROR`] when the server could not be reached.
    pub status: i32,
    /// The body.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// The answer of a request that never reached the server.
    #[must_use]
    pub const fn unreachable() -> Self {
        Self {
            status: RETRYABLE_ERROR,
            body: Vec::new(),
        }
    }
}

/// What sends the requests. It runs on the request's own thread.
pub trait Transport: Send + Sync {
    /// Sends `request` and waits for the answer.
    fn send(&self, request: &HttpRequest) -> HttpResponse;
}

/// The HTTPS transport (rustls, with the certificates built in: no system library needed).
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// A transport with a 30 s limit per request.
    #[must_use]
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for UreqTransport {
    fn send(&self, request: &HttpRequest) -> HttpResponse {
        let result = match &request.post_data {
            Some(form) => self
                .agent
                .post(&request.url)
                .header("User-Agent", &request.user_agent)
                .header(
                    "Content-Type",
                    request
                        .content_type
                        .as_deref()
                        .unwrap_or("application/x-www-form-urlencoded"),
                )
                .send(form.as_bytes()),
            None => self
                .agent
                .get(&request.url)
                .header("User-Agent", &request.user_agent)
                .call(),
        };
        let Ok(mut response) = result else {
            return HttpResponse::unreachable();
        };
        let status = i32::from(response.status().as_u16());
        match response
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .read_to_vec()
        {
            Ok(body) => HttpResponse { status, body },
            Err(_) => HttpResponse::unreachable(),
        }
    }
}
