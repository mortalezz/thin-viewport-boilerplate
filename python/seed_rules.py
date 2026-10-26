"""
seed_rules.py — Initializes policies.db with SQLite schema & FTS5 index.
Creates read-only policies_catalog and policies_fts for zero-write-lock production.
"""

import json
import sqlite3
from pathlib import Path

DB_PATH = Path(__file__).parent / "policies.db"

SEED_POLICIES = [
    {
        "policy_id": "POL-NET-01",
        "name": "Enforce HTTPS Ingress Redirect",
        "category": "Network",
        "severity": "High",
        "description": "Automatically terminates and redirects plain HTTP port 80 requests to encrypted HTTPS port 443 with HSTS headers.",
        "prerequisites": [],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "ingress:\n  tls:\n    enabled: true\n    sslRedirect: true\n    hstsMaxAge: 31536000\n",
    },
    {
        "policy_id": "POL-NET-02",
        "name": "Strict TLS 1.3 Cipher Suites",
        "category": "Network",
        "severity": "Critical",
        "description": "Disallows deprecated TLS 1.0, 1.1, and 1.2 protocols. Enforces modern forward-secret AEAD cipher suites.",
        "prerequisites": ["POL-NET-01"],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "ingress:\n  tls:\n    minVersion: 'TLSv1.3'\n    ciphers: ['TLS_AES_128_GCM_SHA256', 'TLS_AES_256_GCM_SHA384']\n",
    },
    {
        "policy_id": "POL-NET-03",
        "name": "Isolate Private VPC Subnets",
        "category": "Network",
        "severity": "High",
        "description": "Places compute and database clusters into isolated private subnets with NAT gateway egress.",
        "prerequisites": [],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "network:\n  vpc:\n    enablePrivateSubnet: true\n    natGateways: 2\n",
    },
    {
        "policy_id": "POL-NET-04",
        "name": "Deny All Outbound Egress by Default",
        "category": "Network",
        "severity": "Critical",
        "description": "Applies a zero-trust network policy dropping all unsolicited outbound connections from application pods.",
        "prerequisites": ["POL-NET-03"],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "networkPolicy:\n  policyTypes: ['Egress']\n  egress: [] # Blocked by default\n",
    },
    {
        "policy_id": "POL-IAM-01",
        "name": "Require Multi-Factor Authentication (MFA)",
        "category": "Identity",
        "severity": "Critical",
        "description": "Demands hardware FIDO2/WebAuthn or TOTP verification on all administrative sessions.",
        "prerequisites": [],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "iam:\n  mfa:\n    required: true\n    allowedMethods: ['fido2', 'totp']\n",
    },
    {
        "policy_id": "POL-IAM-02",
        "name": "Enforce Zero-Trust Short-Lived Tokens",
        "category": "Identity",
        "severity": "High",
        "description": "Replaces static credentials with ephemeral OIDC/mTLS identity tokens (maximum 15 minutes TTL).",
        "prerequisites": ["POL-IAM-01"],
        "exclusive_group": "AUTH_MODE",
        "exclusive_reason": "Zero-Trust Short-Lived Tokens conflict with Legacy Long-Lived API Keys.",
        "yaml_block": "auth:\n  mode: 'zero-trust'\n  tokenTtlMinutes: 15\n  allowStaticTokens: false\n",
    },
    {
        "policy_id": "POL-IAM-03",
        "name": "Allow Legacy Long-Lived API Keys",
        "category": "Identity",
        "severity": "Low",
        "description": "Permits service accounts to authenticate using permanent pre-shared tokens without rotation.",
        "prerequisites": [],
        "exclusive_group": "AUTH_MODE",
        "exclusive_reason": "Legacy Long-Lived API Keys conflict with Enforce Zero-Trust Short-Lived Tokens.",
        "yaml_block": "auth:\n  mode: 'legacy-static'\n  tokenTtlMinutes: 525600\n  allowStaticTokens: true\n",
    },
    {
        "policy_id": "POL-CMP-01",
        "name": "Disallow Root Container Execution",
        "category": "Compute",
        "severity": "Critical",
        "description": "Blocks any container images configured with UID 0 (root) from deploying to production namespaces.",
        "prerequisites": [],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "securityContext:\n  runAsNonRoot: true\n  runAsUser: 10001\n",
    },
    {
        "policy_id": "POL-CMP-02",
        "name": "Enforce Read-Only Root Filesystem",
        "category": "Compute",
        "severity": "High",
        "description": "Mounts the root container filesystem as strictly read-only; transient writes must use mounted emptyDir tmpfs.",
        "prerequisites": ["POL-CMP-01"],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "securityContext:\n  readOnlyRootFilesystem: true\n",
    },
    {
        "policy_id": "POL-CMP-03",
        "name": "Drop All Linux Capabilities",
        "category": "Compute",
        "severity": "High",
        "description": "Drops all POSIX capabilities (CAP_SYS_ADMIN, CAP_NET_ADMIN) leaving only NET_BIND_SERVICE if needed.",
        "prerequisites": ["POL-CMP-01"],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "securityContext:\n  capabilities:\n    drop: ['ALL']\n",
    },
    {
        "policy_id": "POL-CMP-04",
        "name": "Allow Privileged Container Escape (Debug)",
        "category": "Compute",
        "severity": "Low",
        "description": "Enables privileged mode and host PID namespace access for low-level kernel tracing.",
        "prerequisites": [],
        "exclusive_group": "CONTAINER_PRIVILEGE",
        "exclusive_reason": "Privileged Execution violates Disallow Root Container policies.",
        "yaml_block": "securityContext:\n  privileged: true\n  hostPID: true\n",
    },
    {
        "policy_id": "POL-STO-01",
        "name": "Enforce Encryption at Rest (AES-256)",
        "category": "Storage",
        "severity": "Critical",
        "description": "Mandates transparent block-level volume encryption using hardware-accelerated AES-256-GCM.",
        "prerequisites": [],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "storage:\n  encryption:\n    enabled: true\n    algorithm: 'AES-256-GCM'\n",
    },
    {
        "policy_id": "POL-STO-02",
        "name": "Require Customer-Managed Keys (CMEK)",
        "category": "Storage",
        "severity": "High",
        "description": "Requires disk and bucket encryption keys to be managed in an isolated Cloud KMS / Vault HSM.",
        "prerequisites": ["POL-STO-01"],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "storage:\n  kmsKeyArn: 'arn:aws:kms:us-east-1:123456789012:key/cmek-key-id'\n",
    },
    {
        "policy_id": "POL-STO-03",
        "name": "Deny Public Object Storage Buckets",
        "category": "Storage",
        "severity": "Critical",
        "description": "Enforces cloud provider account-level public access blocks across all S3 and GCS object buckets.",
        "prerequisites": [],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "storage:\n  blockPublicAccess: true\n  restrictPublicBuckets: true\n",
    },
    {
        "policy_id": "POL-GOV-01",
        "name": "Enable Immutable Audit Log Retention",
        "category": "Compliance",
        "severity": "High",
        "description": "Streams API and cluster audit events to write-once-read-many (WORM) storage for 365 days.",
        "prerequisites": [],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "audit:\n  retentionDays: 365\n  wormLockEnabled: true\n",
    },
    {
        "policy_id": "POL-GOV-02",
        "name": "Enforce SOC2 Type II Baseline Controls",
        "category": "Compliance",
        "severity": "Critical",
        "description": "Verifies continuous compliance checks across access controls, encryption, change logs, and monitoring.",
        "prerequisites": ["POL-GOV-01", "POL-IAM-01", "POL-STO-01"],
        "exclusive_group": None,
        "exclusive_reason": None,
        "yaml_block": "compliance:\n  standard: 'SOC2-Type-II'\n  automatedEvidenceCollection: true\n",
    },
]

def seed_db():
    if DB_PATH.exists():
        DB_PATH.unlink()

    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("""
        CREATE TABLE policies_catalog (
            policy_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            severity TEXT NOT NULL,
            description TEXT NOT NULL,
            prerequisites_json TEXT NOT NULL,
            exclusive_group TEXT,
            exclusive_reason TEXT,
            yaml_block TEXT NOT NULL
        )
    """)

    cursor.execute("""
        CREATE VIRTUAL TABLE policies_fts USING fts5(
            policy_id UNINDEXED,
            name,
            category,
            description,
            tokenize='porter unicode61'
        )
    """)

    for p in SEED_POLICIES:
        cursor.execute("""
            INSERT INTO policies_catalog (
                policy_id, name, category, severity, description,
                prerequisites_json, exclusive_group, exclusive_reason, yaml_block
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            p["policy_id"],
            p["name"],
            p["category"],
            p["severity"],
            p["description"],
            json.dumps(p["prerequisites"]),
            p["exclusive_group"],
            p["exclusive_reason"],
            p["yaml_block"]
        ))

        cursor.execute("""
            INSERT INTO policies_fts (policy_id, name, category, description)
            VALUES (?, ?, ?, ?)
        """, (p["policy_id"], p["name"], p["category"], p["description"]))

    conn.commit()
    conn.close()
    print(f"✅ Successfully seeded {len(SEED_POLICIES)} policies into {DB_PATH}")

if __name__ == "__main__":
    seed_db()
