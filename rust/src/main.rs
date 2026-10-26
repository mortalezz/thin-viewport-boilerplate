//! main.rs — Axum HTTP Server & CLI Entrypoint for Thin Viewport Boilerplate.
//! Demonstrates sub-millisecond SSR, zero-write stateless sessions, and CLI micro-benchmarks.

mod catalog;
mod policy_engine;
mod session;
mod views;

use axum::extract::{Form, Multipart, Query};
use axum::http::header::{HeaderMap, HeaderValue, CACHE_CONTROL, CONTENT_TYPE, SET_COOKIE};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Instant;
use tower_http::services::ServeDir;

use catalog::{search_catalog, CATALOG};
use policy_engine::compile_policy_plan;
use session::{decode_policy_cookie, encode_policy_cookie, COOKIE_NAME};
use views::{render_index, render_search_results, render_workspace};

fn extract_policies_from_headers(headers: &HeaderMap) -> Vec<String> {
    if let Some(cookie_hdr) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_hdr.to_str() {
            for pair in cookie_str.split(';') {
                let mut parts = pair.splitn(2, '=');
                if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                    if k.trim() == COOKIE_NAME {
                        return decode_policy_cookie(v.trim());
                    }
                }
            }
        }
    }
    Vec::new()
}

fn make_cookie_header(policies: &[String]) -> String {
    let token = encode_policy_cookie(policies);
    format!(
        "{}={}; HttpOnly; Max-Age={}; Path=/; SameSite=Lax",
        COOKIE_NAME,
        token,
        86400 * 30
    )
}

fn apply_no_cache(res: &mut Response) {
    let h = res.headers_mut();
    h.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    h.insert("Pragma", HeaderValue::from_static("no-cache"));
    h.insert("Vary", HeaderValue::from_static("HX-Request, Cookie"));
}

async fn index_handler(headers: HeaderMap) -> Response {
    let policies = extract_policies_from_headers(&headers);
    let plan = compile_policy_plan(&policies);
    let mut html = String::with_capacity(16384);
    render_index(&mut html, &plan);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    res
}

async fn workspace_handler(headers: HeaderMap) -> Response {
    let policies = extract_policies_from_headers(&headers);
    let plan = compile_policy_plan(&policies);
    let mut html = String::with_capacity(8192);
    render_workspace(&mut html, &plan);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    res
}

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
}

async fn search_handler(headers: HeaderMap, Query(query): Query<SearchQuery>) -> Response {
    let policies = extract_policies_from_headers(&headers);
    let active_set: HashSet<String> = policies.into_iter().collect();

    let mut html = String::with_capacity(4096);
    render_search_results(&mut html, &query.q, &active_set);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    res
}

#[derive(Deserialize)]
struct PolicyForm {
    policy_id: String,
}

async fn toggle_policy_handler(headers: HeaderMap, Form(form): Form<PolicyForm>) -> Response {
    let mut policies = extract_policies_from_headers(&headers);
    if let Some(pos) = policies.iter().position(|x| x == &form.policy_id) {
        policies.remove(pos);
    } else {
        policies.push(form.policy_id);
    }

    let plan = compile_policy_plan(&policies);
    let mut html = String::with_capacity(8192);
    render_workspace(&mut html, &plan);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    let cookie_val = make_cookie_header(&policies);
    if let Ok(val) = HeaderValue::from_str(&cookie_val) {
        res.headers_mut().insert(SET_COOKIE, val);
    }
    res.headers_mut()
        .insert("HX-Trigger", HeaderValue::from_static("refreshSearch"));
    res
}

async fn remove_policy_handler(headers: HeaderMap, Form(form): Form<PolicyForm>) -> Response {
    let mut policies = extract_policies_from_headers(&headers);
    if let Some(pos) = policies.iter().position(|x| x == &form.policy_id) {
        policies.remove(pos);
    }

    let plan = compile_policy_plan(&policies);
    let mut html = String::with_capacity(8192);
    render_workspace(&mut html, &plan);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    let cookie_val = make_cookie_header(&policies);
    if let Ok(val) = HeaderValue::from_str(&cookie_val) {
        res.headers_mut().insert(SET_COOKIE, val);
    }
    res.headers_mut()
        .insert("HX-Trigger", HeaderValue::from_static("refreshSearch"));
    res
}

#[derive(Deserialize)]
struct ConflictForm {
    keep_id: String,
    remove_id: String,
}

async fn resolve_conflict_handler(headers: HeaderMap, Form(form): Form<ConflictForm>) -> Response {
    let mut policies = extract_policies_from_headers(&headers);
    if let Some(pos) = policies.iter().position(|x| x == &form.remove_id) {
        policies.remove(pos);
    }
    if !policies.contains(&form.keep_id) {
        policies.push(form.keep_id);
    }

    let plan = compile_policy_plan(&policies);
    let mut html = String::with_capacity(8192);
    render_workspace(&mut html, &plan);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    let cookie_val = make_cookie_header(&policies);
    if let Ok(val) = HeaderValue::from_str(&cookie_val) {
        res.headers_mut().insert(SET_COOKIE, val);
    }
    res.headers_mut()
        .insert("HX-Trigger", HeaderValue::from_static("refreshSearch"));
    res
}

async fn clear_policies_handler() -> Response {
    let plan = compile_policy_plan(&[]);
    let mut html = String::with_capacity(4096);
    render_workspace(&mut html, &plan);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    let cookie_val = make_cookie_header(&[]);
    if let Ok(val) = HeaderValue::from_str(&cookie_val) {
        res.headers_mut().insert(SET_COOKIE, val);
    }
    res.headers_mut()
        .insert("HX-Trigger", HeaderValue::from_static("refreshSearch"));
    res
}

async fn export_handler(headers: HeaderMap) -> Response {
    let policies = extract_policies_from_headers(&headers);
    let payload = serde_json::json!({
        "version": 1,
        "format": "thin-viewport-policy-v1",
        "active_policies": policies,
    });
    let body = serde_json::to_string_pretty(&payload).unwrap_or_default();

    let mut res = Response::new(body.into());
    res.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    res.headers_mut().insert(
        "Content-Disposition",
        HeaderValue::from_static("attachment; filename=\"policy-snapshot.json\""),
    );
    apply_no_cache(&mut res);
    res
}

async fn import_handler(mut multipart: Multipart) -> Response {
    let mut imported = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Ok(bytes) = field.bytes().await {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(arr) = val.get("active_policies").and_then(|v| v.as_array()) {
                    imported = arr
                        .iter()
                        .filter_map(|s| s.as_str().map(|x| x.to_string()))
                        .collect();
                    break;
                }
            }
        }
    }

    let plan = compile_policy_plan(&imported);
    let mut html = String::with_capacity(8192);
    render_workspace(&mut html, &plan);

    let mut res = Html(html).into_response();
    apply_no_cache(&mut res);
    let cookie_val = make_cookie_header(&imported);
    if let Ok(val) = HeaderValue::from_str(&cookie_val) {
        res.headers_mut().insert(SET_COOKIE, val);
    }
    res.headers_mut()
        .insert("HX-Trigger", HeaderValue::from_static("refreshSearch"));
    res
}

fn run_benchmarks() {
    println!("=========================================================================");
    println!("⚡ Thin Viewport Boilerplate — Rust Engine Micro-Benchmarks");
    println!("=========================================================================");
    println!("Embedded Catalog: {} Policies in .rodata", CATALOG.len());
    println!("-------------------------------------------------------------------------");

    let iters = 20_000;
    let t0 = Instant::now();
    let mut search_hits = 0;
    for _ in 0..iters {
        let res = search_catalog("TLS", 8);
        search_hits += res.len();
    }
    let search_ns = t0.elapsed().as_nanos() as f64 / iters as f64;

    let sample_policies = vec![
        "POL-NET-01".to_string(),
        "POL-NET-02".to_string(),
        "POL-IAM-01".to_string(),
        "POL-CMP-01".to_string(),
        "POL-STO-01".to_string(),
    ];

    let t1 = Instant::now();
    let mut buf = String::with_capacity(16384);
    for _ in 0..iters {
        buf.clear();
        let plan = compile_policy_plan(&sample_policies);
        render_workspace(&mut buf, &plan);
    }
    let render_ns = t1.elapsed().as_nanos() as f64 / iters as f64;

    let t2 = Instant::now();
    for _ in 0..iters {
        let token = encode_policy_cookie(&sample_policies);
        let dec = decode_policy_cookie(&token);
        assert_eq!(dec.len(), 5);
    }
    let cookie_ns = t2.elapsed().as_nanos() as f64 / iters as f64;

    println!(
        "1. In-RAM Policy Prefix Search (`search_catalog('TLS')`)   : {:>6.2} µs ({:.0} ops/sec, {} matches)",
        search_ns / 1000.0,
        1_000_000_000.0 / search_ns,
        search_hits / iters
    );
    println!(
        "2. DAG Solve + HTML Render (5 Active Rules + YAML Manifest): {:>6.2} µs ({:.0} ops/sec)",
        render_ns / 1000.0,
        1_000_000_000.0 / render_ns
    );
    println!(
        "3. v2_ zlib + base64url Cookie Encode/Decode Roundtrip    : {:>6.2} µs ({:.0} ops/sec)",
        cookie_ns / 1000.0,
        1_000_000_000.0 / cookie_ns
    );
    println!("=========================================================================");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--bench") {
        run_benchmarks();
        return;
    }

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    let dist_assets = PathBuf::from("../dist/assets");
    let asset_service = if dist_assets.exists() {
        ServeDir::new(dist_assets)
    } else {
        ServeDir::new("dist/assets")
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/workspace", get(workspace_handler))
        .route("/search", get(search_handler))
        .route("/policy/toggle", post(toggle_policy_handler))
        .route("/policy/remove", post(remove_policy_handler))
        .route("/policy/resolve-conflict", post(resolve_conflict_handler))
        .route("/policy/clear", post(clear_policies_handler))
        .route("/export", get(export_handler))
        .route("/import", post(import_handler))
        .nest_service("/assets", asset_service);

    println!("=========================================================================");
    println!("⚡ Thin Viewport Boilerplate (Axum Rust Server)");
    println!("=========================================================================");
    println!("Listening on http://127.0.0.1:{}", port);
    println!("Tip: Run with `cargo run --release -- --bench` to test microsecond execution");
    println!("=========================================================================");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
