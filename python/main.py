"""
main.py — FastAPI application demonstrating the Thin Viewport & Stateless Cookie Architecture.
Features:
- Ephemeral, zero-runtime-write SQLite FTS5 search
- v2_ zlib + base64url HTTP-only session cookies
- hx-sync mutation serialization
- Fast SSR with Jinja2 partials
- Portable JSON snapshot export/import
"""

import base64
import json
import zlib
from pathlib import Path
from typing import Optional

from fastapi import FastAPI, Form, HTTPException, Request, Response, UploadFile, File
from fastapi.responses import HTMLResponse, JSONResponse, StreamingResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

from policy_engine import compile_policy_plan, search_policies

BASE_DIR = Path(__file__).resolve().parent
DIST_DIR = BASE_DIR / "dist" if (BASE_DIR / "dist").exists() else BASE_DIR.parent / "dist"
INDEX_HTML_PATH = DIST_DIR / "index.html"
TEMPLATES_DIR = BASE_DIR / "templates"

app = FastAPI(title="Thin Viewport & Stateless Cookie Boilerplate")

if (DIST_DIR / "assets").exists():
    app.mount("/assets", StaticFiles(directory=str(DIST_DIR / "assets")), name="assets")

templates = Jinja2Templates(directory=str(TEMPLATES_DIR))

# --- Cache Guardrails & Cookie Codec ---
EDGE_CACHE_HEADERS = {
    "Cache-Control": "public, max-age=3600, s-maxage=86400, stale-while-revalidate=86400",
    "Vary": "Accept-Encoding",
}

NO_CACHE_HEADERS = {
    "Cache-Control": "no-store, no-cache, must-revalidate, max-age=0",
    "Pragma": "no-cache",
    "Vary": "HX-Request, Cookie",
}

COOKIE_NAME = "tv_policies"
COOKIE_MAX_AGE = 86400 * 30  # 30 days
MAX_DECOMPRESSED_COOKIE_BYTES = 65536

def decode_policy_cookie(raw_val: Optional[str]) -> list[str]:
    if not raw_val:
        return []
    try:
        if raw_val.startswith("v2_"):
            token = raw_val[3:]
            rem = len(token) % 4
            if rem > 0:
                token += "=" * (4 - rem)
            compressed = base64.urlsafe_b64decode(token.encode("ascii"))
            decomp_obj = zlib.decompressobj()
            decompressed = decomp_obj.decompress(compressed, MAX_DECOMPRESSED_COOKIE_BYTES)
            if decomp_obj.unconsumed_tail:
                return []
            text = decompressed.decode("utf-8")
            return [line.strip() for line in text.splitlines() if line.strip()]
        else:
            # Fallback legacy JSON
            data = json.loads(raw_val)
            if isinstance(data, list):
                return [str(x) for x in data if isinstance(x, str)]
    except Exception:
        return []
    return []

def encode_policy_cookie(policies: list[str]) -> str:
    text = "\n".join(policies)
    compressed = zlib.compress(text.encode("utf-8"), level=9)
    b64 = base64.urlsafe_b64encode(compressed).decode("ascii").rstrip("=")
    return f"v2_{b64}"

def render_workspace_response(request: Request, policies: list[str], refresh_search: bool = False) -> Response:
    plan = compile_policy_plan(policies)
    response = templates.TemplateResponse(
        "partials/workspace.html",
        {"request": request, "plan": plan},
        headers=NO_CACHE_HEADERS,
    )
    cookie_val = encode_policy_cookie(policies)
    response.set_cookie(
        key=COOKIE_NAME,
        value=cookie_val,
        max_age=COOKIE_MAX_AGE,
        httponly=True,
        samesite="lax",
    )
    if refresh_search:
        response.headers["HX-Trigger"] = "refreshSearch"
    return response

# --- HTTP Endpoints ---

@app.get("/api/health")
async def health_check():
    """Zero-allocation keep-warm and healthcheck endpoint."""
    return {"status": "ok"}

@app.api_route("/", methods=["GET", "HEAD"], response_class=HTMLResponse)
@app.api_route("/index.html", methods=["GET", "HEAD"], response_class=HTMLResponse)
async def get_index(request: Request):
    if INDEX_HTML_PATH.exists():
        return HTMLResponse(content=INDEX_HTML_PATH.read_text(encoding="utf-8"), headers=EDGE_CACHE_HEADERS)
    raw_cookie = request.cookies.get(COOKIE_NAME)
    policies = decode_policy_cookie(raw_cookie)
    plan = compile_policy_plan(policies)
    return templates.TemplateResponse(
        "index.html",
        {"request": request, "plan": plan},
        headers=NO_CACHE_HEADERS,
    )

@app.get("/workspace", response_class=HTMLResponse)
async def get_workspace(request: Request):
    raw_cookie = request.cookies.get(COOKIE_NAME)
    policies = decode_policy_cookie(raw_cookie)
    return render_workspace_response(request, policies)

@app.get("/search", response_class=HTMLResponse)
async def search_typeahead(request: Request, q: str = ""):
    raw_cookie = request.cookies.get(COOKIE_NAME)
    active_policies = set(decode_policy_cookie(raw_cookie))
    results = search_policies(q, limit=8) if q.strip() else []
    return templates.TemplateResponse(
        "partials/search_results.html",
        {"request": request, "results": results, "active_policies": active_policies, "q": q},
        headers=NO_CACHE_HEADERS,
    )

@app.post("/policy/toggle", response_class=HTMLResponse)
async def toggle_policy(request: Request, policy_id: str = Form(...)):
    raw_cookie = request.cookies.get(COOKIE_NAME)
    policies = decode_policy_cookie(raw_cookie)
    if policy_id in policies:
        policies.remove(policy_id)
    else:
        policies.append(policy_id)
    return render_workspace_response(request, policies, refresh_search=True)

@app.post("/policy/remove", response_class=HTMLResponse)
async def remove_policy(request: Request, policy_id: str = Form(...)):
    raw_cookie = request.cookies.get(COOKIE_NAME)
    policies = decode_policy_cookie(raw_cookie)
    if policy_id in policies:
        policies.remove(policy_id)
    return render_workspace_response(request, policies, refresh_search=True)

@app.post("/policy/resolve-conflict", response_class=HTMLResponse)
async def resolve_conflict(request: Request, keep_id: str = Form(...), remove_id: str = Form(...)):
    raw_cookie = request.cookies.get(COOKIE_NAME)
    policies = decode_policy_cookie(raw_cookie)
    if remove_id in policies:
        policies.remove(remove_id)
    if keep_id not in policies:
        policies.append(keep_id)
    return render_workspace_response(request, policies, refresh_search=True)

@app.post("/policy/clear", response_class=HTMLResponse)
async def clear_policies(request: Request):
    return render_workspace_response(request, [], refresh_search=True)

@app.get("/export")
async def export_snapshot(request: Request):
    raw_cookie = request.cookies.get(COOKIE_NAME)
    policies = decode_policy_cookie(raw_cookie)
    payload = {
        "version": 1,
        "format": "thin-viewport-policy-v1",
        "active_policies": policies,
    }
    content = json.dumps(payload, indent=2)
    return Response(
        content=content,
        media_type="application/json",
        headers={
            "Content-Disposition": 'attachment; filename="policy-snapshot.json"',
            **NO_CACHE_HEADERS,
        },
    )

@app.post("/import", response_class=HTMLResponse)
async def import_snapshot(request: Request, file: UploadFile = File(...)):
    try:
        content = await file.read()
        data = json.loads(content.decode("utf-8"))
        if not isinstance(data, dict) or "active_policies" not in data:
            raise ValueError("Invalid format")
        imported_policies = [str(x) for x in data["active_policies"] if isinstance(x, str)]
        return render_workspace_response(request, imported_policies, refresh_search=True)
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Failed to import policy snapshot: {str(e)}")


# Low-priority static frontend routes with Cloudflare Edge CDN caching on FastAPI Cloud
if hasattr(app, "frontend") and DIST_DIR.exists():
    app.frontend("/", directory=DIST_DIR)
elif DIST_DIR.exists():
    app.mount("/", StaticFiles(directory=str(DIST_DIR), html=True), name="frontend")
