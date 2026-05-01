ADI_PORT = 5151
ARX_PORT = 5152
APM_PORT = 5153
GATEWAY_PORT = 5100

SECURITY = {
    "oauth2": {"issuer": "https://auth.aetheros.local", "audience": "aetheros-coder"},
    "jwt": {"algorithm": "RS256", "jwks_url": "https://auth.aetheros.local/.well-known/jwks.json"},
    "tls": {"enabled": True, "min_version": "TLSv1.2"},
    "rbac": {"enabled": True, "policy_source": "policy/rbac.yaml"},
    "api_keys": {"enabled": False},
}
