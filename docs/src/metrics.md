# Metrics

Proxima exposes a `/-/metrics` path that can be scraped by Prometheus. 

The following metric types are currently enabled:
- TYPE proxima_cache_attempt_total counter
- TYPE proxima_cache_keys gauge
- TYPE proxima_cache_miss_total counter
- TYPE proxima_config_renew_attempts_total counter
- TYPE proxima_config_renew_failures_total counter
- TYPE proxima_jwts_renew_attempts_total counter
- TYPE proxima_jwts_renew_failures_total counter
- TYPE proxima_requests_duration_seconds histogram
- TYPE proxima_requests_total counter
- TYPE proxima_response_errors_total counter
- TYPE proxima_security_client_authentication_total counter
- TYPE proxima_security_method_attempts_total counter
- TYPE proxima_security_method_blocked_total counter
