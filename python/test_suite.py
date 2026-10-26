import json
from fastapi.testclient import TestClient
from main import app, decode_policy_cookie, encode_policy_cookie

client = TestClient(app)

def test_full_pipeline():
    print("1. Testing GET / (Shell)...")
    res = client.get("/")
    assert res.status_code == 200
    assert "Thin Viewport" in res.text

    print("2. Testing FTS5 Search (GET /search?q=TLS)...")
    res = client.get("/search?q=TLS")
    assert res.status_code == 200
    assert "Strict TLS 1.3" in res.text

    print("3. Testing POST /policy/toggle (State Mutation & v2_ Cookie)...")
    res = client.post("/policy/toggle", data={"policy_id": "POL-NET-02"})
    assert res.status_code == 200
    cookie_hdr = res.headers.get("set-cookie")
    assert cookie_hdr is not None
    assert "tv_policies=v2_" in cookie_hdr

    # Extract cookie
    raw_val = res.cookies.get("tv_policies")
    active = decode_policy_cookie(raw_val)
    assert active == ["POL-NET-02"]

    print("4. Testing Conflict Detection (POL-IAM-02 vs POL-IAM-03)...")
    # Add POL-IAM-02
    res2 = client.post("/policy/toggle", data={"policy_id": "POL-IAM-02"}, cookies={"tv_policies": raw_val})
    raw_val2 = res2.cookies.get("tv_policies")
    # Add conflicting POL-IAM-03
    res3 = client.post("/policy/toggle", data={"policy_id": "POL-IAM-03"}, cookies={"tv_policies": raw_val2})
    assert "Mutual Exclusivity Conflict Detected" in res3.text
    assert "Keep Enforce Zero-Trust Short-Lived Tokens" in res3.text

    print("5. Testing Conflict Resolution...")
    raw_val3 = res3.cookies.get("tv_policies")
    res4 = client.post(
        "/policy/resolve-conflict",
        data={"keep_id": "POL-IAM-02", "remove_id": "POL-IAM-03"},
        cookies={"tv_policies": raw_val3}
    )
    assert "Mutual Exclusivity Conflict Detected" not in res4.text
    resolved = decode_policy_cookie(res4.cookies.get("tv_policies"))
    assert "POL-IAM-02" in resolved
    assert "POL-IAM-03" not in resolved

    print("6. Testing Export & Import...")
    exp_res = client.get("/export", cookies={"tv_policies": res4.cookies.get("tv_policies")})
    assert exp_res.status_code == 200
    payload = exp_res.json()
    assert "active_policies" in payload
    assert "POL-IAM-02" in payload["active_policies"]

    # Import
    import_bytes = json.dumps(payload).encode("utf-8")
    imp_res = client.post("/import", files={"file": ("test.json", import_bytes, "application/json")})
    assert imp_res.status_code == 200
    assert "POL-IAM-02" in imp_res.text

    print("✅ All 6 Python Test Pipeline Steps Passed!")

if __name__ == "__main__":
    test_full_pipeline()
