# Metrics

Proxima exposes a `/-/metrics` path that can be scraped by Prometheus. 

The following metric types are currently enabled:
- TYPE proxima_cache_attempts_total counter
- TYPE proxima_cache_hit_total counter
- TYPE proxima_config_renew_attempts_total counter
- TYPE proxima_config_renew_success_total counter
- TYPE proxima_endpoint_authentication_total counter
- TYPE proxima_endpoint_authentication_basic_failed_total counter
- TYPE proxima_endpoint_authentication_digest_failed_total counter
- TYPE proxima_endpoint_authentication_token_failed_total counter
- TYPE proxima_requests_duration_seconds histogram
- TYPE proxima_requests_total counter

