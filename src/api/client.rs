use log::{warn, error};
use reqwest::{StatusCode, header::HeaderMap};
use std::{time::{Duration, Instant}, fmt};
use tokio::sync::Mutex;
use regex::Regex;

pub struct RateLimitPolicy {
    pub max_requests: u64,
    pub time_window: u64
}

impl RateLimitPolicy {
    pub fn default() -> Self {
        RateLimitPolicy { max_requests: 50, time_window: 30 }
    }
}

#[derive(Debug)]
pub enum ApiError {
    RateLimit(Duration),
    RequestError(reqwest::Error),
    TimedOut,
    NotFound,
    ServerError,
}

impl std::error::Error for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::RateLimit(duration) => {
                write!(f, "Rate limited, resets in {} seconds", duration.as_secs())
            },
            Self::RequestError(err) => {
                write!(f, "Request error {}", err)
            },
            Self::TimedOut => {
                write!(f, "Request timed out")
            },
            Self::NotFound => {
                write!(f, "Requested entity does not exist")
            },
            Self::ServerError => {
                write!(f, "API server error")
            }
        }
    }
}

pub struct RateLimit {
    pub policy: RateLimitPolicy,
    pub limit: u64,
    pub remaining: u64,
    pub reset: u64,
    pub rollover: Option<Instant>,
}

lazy_static::lazy_static! {
    static ref POLICY_RE: Regex = Regex::new(r"^([0-9]+);w=([0-9]+)$").unwrap();
}

fn parse_numeric_header(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers.get(name)
    .and_then(|v| v.to_str().ok())
    .and_then(|s| s.parse().ok())
}

impl RateLimit {
    fn check_rate_limit(&mut self) -> Result<(), ApiError> {
        if let Some(rollover) = self.rollover 
            && Instant::now() > rollover {
            self.rollover = None;
            self.remaining = self.policy.max_requests;
            self.reset = self.policy.time_window;
        }

        if self.remaining < 5 {
            warn!("Too close to rate limit; waiting for time window to roll over");

            return Err(ApiError::RateLimit(
                match &self.rollover {
                    Some(rollover) => rollover.duration_since(Instant::now()),
                    None => Duration::from_secs(self.policy.time_window)
                }
            ));
        }

        Ok(())
    }

    fn update_rate_limit(&mut self, headers: &HeaderMap) {
        if let Some(captures) = 
            headers.get("ratelimit-policy").and_then(
                |v| v.to_str().ok()
            ).and_then(|s| POLICY_RE.captures(s)) {
            if let Ok(v) = captures[1].parse() { self.policy.max_requests = v; }
            if let Ok(v) = captures[2].parse() { self.policy.time_window = v; }
        }

        if let Some(limit) = parse_numeric_header(headers, "ratelimit-limit") {
            self.limit = limit;
        }

        if let Some(remaining) = parse_numeric_header(headers, "ratelimit-remaining") {
            self.remaining = remaining;
        }

        if let Some(reset) = parse_numeric_header(headers, "ratelimit-reset") {
            self.reset = reset;
            self.rollover = Instant::now().checked_add(Duration::from_secs(reset));
        }
    }
}

pub struct Client {
    pub user_agent: String,
    pub client: reqwest::Client,
    pub limit: Mutex<RateLimit>,
}

impl Client {
    pub fn new(user_agent: String) -> Result<Self, reqwest::Error> {
        Ok(Client { 
            user_agent, 
            client: reqwest::ClientBuilder::new().timeout(Duration::from_secs(10)).build()?,
            limit: Mutex::new(RateLimit {
                policy: RateLimitPolicy::default(),
                limit: 50,
                remaining: 50,
                reset: 30,
                rollover: None,
            })
        })
    }

    const API_URL: &'static str = "https://www.nationstates.net/cgi-bin/api.cgi";

    pub async fn make_request(
        &self, params: Vec<(&str, &str)>
    ) -> Result<String, ApiError> {
        let mut limit = self.limit.lock().await;

        limit.check_rate_limit()?;

        let response = match 
            self.client.get(Self::API_URL)
            .query(&params)
            .header("User-Agent", self.user_agent.clone())
            .send().await {
                Ok(r) => r,
                Err(err) => {
                    if err.is_timeout() { 
                        warn!("API request timed out");
                        return Err(ApiError::TimedOut);
                    }

                    error!("API request returned error {}", err);
                    return Err(ApiError::RequestError(err));
                }
        };

        let headers = response.headers();

        limit.update_rate_limit(headers);

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            warn!("Hit rate limit: 429 Too Many Requests");

            return Err(ApiError::RateLimit(
                match parse_numeric_header(headers, "retry-after") {
                    Some(v) => Duration::from_secs(v),
                    None => Duration::from_secs(limit.policy.time_window)
                }
            ));
        }

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ApiError::NotFound);
        }

        if response.status().is_server_error() {
            return Err(ApiError::ServerError);
        }

        match response.text().await {
            Ok(t) => Ok(t),
            Err(e) => Err(ApiError::RequestError(e))
        }
    }
}