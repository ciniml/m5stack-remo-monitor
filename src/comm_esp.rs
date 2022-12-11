use std::fmt::Write;

use crate::{RateLimitInfo, ACCESS_TOKEN_BEARER_LENGTH, config::ACCESS_TOKEN};

use esp_idf_svc::{netif::EspNetifStack, wifi::EspWifi, timer::{EspTimerService, EspTaskTimerService, EspTimer}};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use embedded_svc::http::Headers;

pub fn fetch_http_and_parse<F, ParserResult>(url: &str, mut response_parser: F) -> anyhow::Result<(ParserResult, RateLimitInfo)> 
    where F: for <'a> FnMut(esp_idf_svc::http::client::EspHttpResponse<'a>) -> anyhow::Result<ParserResult>
{
    use embedded_svc::http::{client::*};
    use esp_idf_svc::http::client::*;

    let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut access_token = heapless::String::<ACCESS_TOKEN_BEARER_LENGTH>::new();
    access_token.write_str("Bearer ").unwrap();
    access_token.write_str(ACCESS_TOKEN).unwrap();
    let mut request = client.get(&url)?;
    request.set_header("Authorization", &access_token.as_str());
    let response = request.submit()?;
    
    // x-rate-limit-limit: 30
    // x-rate-limit-remaining: 25
    // x-rate-limit-reset: 1667922300
    // x-xss-protection: 1; mode=block
    let rate_limit = RateLimitInfo {
        limit: response.header("x-rate-limit-limit").and_then(|v| v.parse().ok()),
        remaining: response.header("x-rate-limit-remaining").and_then(|v| v.parse().ok()),
        reset: response.header("x-rate-limit-reset").and_then(|v| v.parse().ok()),
    };
    let result = response_parser(response)?;
    Ok((result, rate_limit))
}
