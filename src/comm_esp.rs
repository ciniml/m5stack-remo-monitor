use std::fmt::Write;

use crate::{RateLimitInfo};

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

pub const MAX_ACCESS_TOKEN_LEN: usize = 128;
pub const ACCESS_TOKEN_BEARER_LENGTH: usize = "Bearer ".len() + MAX_ACCESS_TOKEN_LEN;

pub fn fetch_http_and_parse<F, ParserResult>(url: &str, access_token: &str, mut response_parser: F) -> anyhow::Result<(ParserResult, RateLimitInfo)> 
    where F: FnMut(esp_idf_svc::http::client::EspHttpConnection) -> anyhow::Result<ParserResult>
{   
    use esp_idf_svc::http::client::*;

    let mut client = EspHttpConnection::new(&Configuration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut autiorization_header = heapless::String::<ACCESS_TOKEN_BEARER_LENGTH>::new();
    autiorization_header.write_str("Bearer ").unwrap();
    autiorization_header.write_str(access_token).unwrap();
    let headers = [("Authorization", autiorization_header.as_str())];
    client.initiate_request(embedded_svc::http::Method::Get, &url, &headers)?;
    client.initiate_response()?;

    // x-rate-limit-limit: 30
    // x-rate-limit-remaining: 25
    // x-rate-limit-reset: 1667922300
    // x-xss-protection: 1; mode=block
    let rate_limit = RateLimitInfo {
        limit: client.header("x-rate-limit-limit").and_then(|v| v.parse().ok()),
        remaining: client.header("x-rate-limit-remaining").and_then(|v| v.parse().ok()),
        reset: client.header("x-rate-limit-reset").and_then(|v| v.parse().ok()),
    };
    let result = response_parser(client)?;
    Ok((result, rate_limit))
}
