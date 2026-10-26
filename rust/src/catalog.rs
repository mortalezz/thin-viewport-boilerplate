//! catalog.rs — Static policy catalog embedded directly into .rodata.
//! Zero heap allocation on catalog read, zero runtime database locks.

#[derive(Debug, Clone, Copy)]
pub struct PolicyItem {
    pub policy_id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub severity: &'static str,
    pub description: &'static str,
    pub prerequisites: &'static [&'static str],
    pub exclusive_group: Option<&'static str>,
    pub exclusive_reason: Option<&'static str>,
    pub yaml_block: &'static str,
}

pub static CATALOG: &[PolicyItem] = &[
    PolicyItem {
        policy_id: "POL-NET-01",
        name: "Enforce HTTPS Ingress Redirect",
        category: "Network",
        severity: "High",
        description: "Automatically terminates and redirects plain HTTP port 80 requests to encrypted HTTPS port 443 with HSTS headers.",
        prerequisites: &[],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "ingress:\n  tls:\n    enabled: true\n    sslRedirect: true\n    hstsMaxAge: 31536000",
    },
    PolicyItem {
        policy_id: "POL-NET-02",
        name: "Strict TLS 1.3 Cipher Suites",
        category: "Network",
        severity: "Critical",
        description: "Disallows deprecated TLS 1.0, 1.1, and 1.2 protocols. Enforces modern forward-secret AEAD cipher suites.",
        prerequisites: &["POL-NET-01"],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "ingress:\n  tls:\n    minVersion: 'TLSv1.3'\n    ciphers: ['TLS_AES_128_GCM_SHA256', 'TLS_AES_256_GCM_SHA384']",
    },
    PolicyItem {
        policy_id: "POL-NET-03",
        name: "Isolate Private VPC Subnets",
        category: "Network",
        severity: "High",
        description: "Places compute and database clusters into isolated private subnets with NAT gateway egress.",
        prerequisites: &[],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "network:\n  vpc:\n    enablePrivateSubnet: true\n    natGateways: 2",
    },
    PolicyItem {
        policy_id: "POL-NET-04",
        name: "Deny All Outbound Egress by Default",
        category: "Network",
        severity: "Critical",
        description: "Applies a zero-trust network policy dropping all unsolicited outbound connections from application pods.",
        prerequisites: &["POL-NET-03"],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "networkPolicy:\n  policyTypes: ['Egress']\n  egress: [] # Blocked by default",
    },
    PolicyItem {
        policy_id: "POL-IAM-01",
        name: "Require Multi-Factor Authentication (MFA)",
        category: "Identity",
        severity: "Critical",
        description: "Demands hardware FIDO2/WebAuthn or TOTP verification on all administrative sessions.",
        prerequisites: &[],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "iam:\n  mfa:\n    required: true\n    allowedMethods: ['fido2', 'totp']",
    },
    PolicyItem {
        policy_id: "POL-IAM-02",
        name: "Enforce Zero-Trust Short-Lived Tokens",
        category: "Identity",
        severity: "High",
        description: "Replaces static credentials with ephemeral OIDC/mTLS identity tokens (maximum 15 minutes TTL).",
        prerequisites: &["POL-IAM-01"],
        exclusive_group: Some("AUTH_MODE"),
        exclusive_reason: Some("Zero-Trust Short-Lived Tokens conflict with Legacy Long-Lived API Keys."),
        yaml_block: "auth:\n  mode: 'zero-trust'\n  tokenTtlMinutes: 15\n  allowStaticTokens: false",
    },
    PolicyItem {
        policy_id: "POL-IAM-03",
        name: "Allow Legacy Long-Lived API Keys",
        category: "Identity",
        severity: "Low",
        description: "Permits service accounts to authenticate using permanent pre-shared tokens without rotation.",
        prerequisites: &[],
        exclusive_group: Some("AUTH_MODE"),
        exclusive_reason: Some("Legacy Long-Lived API Keys conflict with Enforce Zero-Trust Short-Lived Tokens."),
        yaml_block: "auth:\n  mode: 'legacy-static'\n  tokenTtlMinutes: 525600\n  allowStaticTokens: true",
    },
    PolicyItem {
        policy_id: "POL-CMP-01",
        name: "Disallow Root Container Execution",
        category: "Compute",
        severity: "Critical",
        description: "Blocks any container images configured with UID 0 (root) from deploying to production namespaces.",
        prerequisites: &[],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "securityContext:\n  runAsNonRoot: true\n  runAsUser: 10001",
    },
    PolicyItem {
        policy_id: "POL-CMP-02",
        name: "Enforce Read-Only Root Filesystem",
        category: "Compute",
        severity: "High",
        description: "Mounts the root container filesystem as strictly read-only; transient writes must use mounted emptyDir tmpfs.",
        prerequisites: &["POL-CMP-01"],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "securityContext:\n  readOnlyRootFilesystem: true",
    },
    PolicyItem {
        policy_id: "POL-CMP-03",
        name: "Drop All Linux Capabilities",
        category: "Compute",
        severity: "High",
        description: "Drops all POSIX capabilities (CAP_SYS_ADMIN, CAP_NET_ADMIN) leaving only NET_BIND_SERVICE if needed.",
        prerequisites: &["POL-CMP-01"],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "securityContext:\n  capabilities:\n    drop: ['ALL']",
    },
    PolicyItem {
        policy_id: "POL-CMP-04",
        name: "Allow Privileged Container Escape (Debug)",
        category: "Compute",
        severity: "Low",
        description: "Enables privileged mode and host PID namespace access for low-level kernel tracing.",
        prerequisites: &[],
        exclusive_group: Some("CONTAINER_PRIVILEGE"),
        exclusive_reason: Some("Privileged Execution violates Disallow Root Container policies."),
        yaml_block: "securityContext:\n  privileged: true\n  hostPID: true",
    },
    PolicyItem {
        policy_id: "POL-STO-01",
        name: "Enforce Encryption at Rest (AES-256)",
        category: "Storage",
        severity: "Critical",
        description: "Mandates transparent block-level volume encryption using hardware-accelerated AES-256-GCM.",
        prerequisites: &[],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "storage:\n  encryption:\n    enabled: true\n    algorithm: 'AES-256-GCM'",
    },
    PolicyItem {
        policy_id: "POL-STO-02",
        name: "Require Customer-Managed Keys (CMEK)",
        category: "Storage",
        severity: "High",
        description: "Requires disk and bucket encryption keys to be managed in an isolated Cloud KMS / Vault HSM.",
        prerequisites: &["POL-STO-01"],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "storage:\n  kmsKeyArn: 'arn:aws:kms:us-east-1:123456789012:key/cmek-key-id'",
    },
    PolicyItem {
        policy_id: "POL-STO-03",
        name: "Deny Public Object Storage Buckets",
        category: "Storage",
        severity: "Critical",
        description: "Enforces cloud provider account-level public access blocks across all S3 and GCS object buckets.",
        prerequisites: &[],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "storage:\n  blockPublicAccess: true\n  restrictPublicBuckets: true",
    },
    PolicyItem {
        policy_id: "POL-GOV-01",
        name: "Enable Immutable Audit Log Retention",
        category: "Compliance",
        severity: "High",
        description: "Streams API and cluster audit events to write-once-read-many (WORM) storage for 365 days.",
        prerequisites: &[],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "audit:\n  retentionDays: 365\n  wormLockEnabled: true",
    },
    PolicyItem {
        policy_id: "POL-GOV-02",
        name: "Enforce SOC2 Type II Baseline Controls",
        category: "Compliance",
        severity: "Critical",
        description: "Verifies continuous compliance checks across access controls, encryption, change logs, and monitoring.",
        prerequisites: &["POL-GOV-01", "POL-IAM-01", "POL-STO-01"],
        exclusive_group: None,
        exclusive_reason: None,
        yaml_block: "compliance:\n  standard: 'SOC2-Type-II'\n  automatedEvidenceCollection: true",
    },
];

pub fn find_policy(id: &str) -> Option<&'static PolicyItem> {
    CATALOG.iter().find(|p| p.policy_id == id)
}

pub fn search_catalog(query: &str, limit: usize) -> Vec<&'static PolicyItem> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let tokens: Vec<&str> = q.split_whitespace().collect();
    let mut matches = Vec::new();

    for item in CATALOG {
        let name_lower = item.name.to_ascii_lowercase();
        let cat_lower = item.category.to_ascii_lowercase();
        let desc_lower = item.description.to_ascii_lowercase();
        let id_lower = item.policy_id.to_ascii_lowercase();

        let all_match = tokens.iter().all(|t| {
            name_lower.contains(t) || cat_lower.contains(t) || desc_lower.contains(t) || id_lower.contains(t)
        });

        if all_match {
            matches.push(item);
            if matches.len() >= limit {
                break;
            }
        }
    }
    matches
}
