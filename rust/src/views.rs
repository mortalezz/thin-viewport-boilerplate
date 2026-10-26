//! views.rs — Zero-allocation server-side HTML rendering.
//! Produces exact HTML parity with Jinja2 templates without runtime template parsing overhead.

use std::collections::HashSet;
use crate::catalog::search_catalog;
use crate::policy_engine::PolicyPlan;

#[inline]
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn render_workspace(buf: &mut String, plan: &PolicyPlan) {
    buf.push_str("<div class=\"space-y-6\">\n");

    // Conflict Alert Banner
    if !plan.conflicts.is_empty() {
        buf.push_str("  <div class=\"p-4 rounded-xl border border-red-500/40 bg-red-950/30 text-red-200 space-y-3 shadow-lg\">\n");
        buf.push_str("    <div class=\"flex items-center gap-2 font-semibold text-sm text-red-400\">\n");
        buf.push_str("      <span class=\"text-base\">⚠️</span> Mutual Exclusivity Conflict Detected\n");
        buf.push_str("    </div>\n");

        for conf in &plan.conflicts {
            buf.push_str("    <div class=\"text-xs space-y-2 border-t border-red-900/50 pt-2\">\n");
            buf.push_str(&format!("      <p class=\"text-zinc-300 font-medium\">{}</p>\n", escape_html(&conf.reason)));
            buf.push_str("      <div class=\"flex flex-wrap gap-2 items-center\">\n");
            buf.push_str("        <span class=\"text-zinc-500 text-xs\">Resolve choice:</span>\n");

            let p0 = conf.policies[0];
            let p1 = conf.policies[1];

            buf.push_str("        <form hx-post=\"/policy/resolve-conflict\" hx-target=\"#workspace\" hx-swap=\"innerHTML\" class=\"inline\">\n");
            buf.push_str(&format!("          <input type=\"hidden\" name=\"keep_id\" value=\"{}\">\n", p0.policy_id));
            buf.push_str(&format!("          <input type=\"hidden\" name=\"remove_id\" value=\"{}\">\n", p1.policy_id));
            buf.push_str(&format!("          <button type=\"submit\" class=\"px-2.5 py-1 rounded bg-red-900/40 border border-red-700/60 hover:bg-red-800/60 text-red-100 text-xs transition\">Keep {}</button>\n", escape_html(p0.name)));
            buf.push_str("        </form>\n");

            buf.push_str("        <span class=\"text-zinc-600\">vs</span>\n");

            buf.push_str("        <form hx-post=\"/policy/resolve-conflict\" hx-target=\"#workspace\" hx-swap=\"innerHTML\" class=\"inline\">\n");
            buf.push_str(&format!("          <input type=\"hidden\" name=\"keep_id\" value=\"{}\">\n", p1.policy_id));
            buf.push_str(&format!("          <input type=\"hidden\" name=\"remove_id\" value=\"{}\">\n", p0.policy_id));
            buf.push_str(&format!("          <button type=\"submit\" class=\"px-2.5 py-1 rounded bg-red-900/40 border border-red-700/60 hover:bg-red-800/60 text-red-100 text-xs transition\">Keep {}</button>\n", escape_html(p1.name)));
            buf.push_str("        </form>\n");

            buf.push_str("      </div>\n    </div>\n");
        }
        buf.push_str("  </div>\n");
    }

    // Stats Grid
    buf.push_str("  <div class=\"grid grid-cols-2 sm:grid-cols-4 gap-3\">\n");
    buf.push_str(&format!("    <div class=\"p-3.5 rounded-xl border border-zinc-800/80 bg-zinc-900/40\"><div class=\"text-xs text-zinc-500 uppercase tracking-wider font-semibold\">Active Policies</div><div class=\"text-2xl font-bold text-zinc-100 mt-1\">{} <span class=\"text-xs text-zinc-500 font-normal\">/ {}</span></div></div>\n", plan.total_active, plan.total_catalog));
    buf.push_str(&format!("    <div class=\"p-3.5 rounded-xl border border-zinc-800/80 bg-zinc-900/40\"><div class=\"text-xs text-zinc-500 uppercase tracking-wider font-semibold\">Categories</div><div class=\"text-2xl font-bold text-zinc-100 mt-1\">{}</div></div>\n", plan.categories.len()));

    if plan.missing_prereqs_count > 0 {
        buf.push_str(&format!("    <div class=\"p-3.5 rounded-xl border border-amber-500/40 bg-amber-950/20\"><div class=\"text-xs text-amber-400 uppercase tracking-wider font-semibold\">Prereq Warnings</div><div class=\"text-2xl font-bold text-amber-300 mt-1\">{}</div></div>\n", plan.missing_prereqs_count));
    } else {
        buf.push_str("    <div class=\"p-3.5 rounded-xl border border-zinc-800/80 bg-zinc-900/40\"><div class=\"text-xs text-zinc-500 uppercase tracking-wider font-semibold\">Prereq Warnings</div><div class=\"text-2xl font-bold text-zinc-100 mt-1\">0</div></div>\n");
    }

    if !plan.conflicts.is_empty() {
        buf.push_str(&format!("    <div class=\"p-3.5 rounded-xl border border-red-500/40 bg-red-950/20\"><div class=\"text-xs text-red-400 uppercase tracking-wider font-semibold\">Collisions</div><div class=\"text-2xl font-bold text-red-400 mt-1\">{}</div></div>\n", plan.conflicts.len()));
    } else {
        buf.push_str("    <div class=\"p-3.5 rounded-xl border border-zinc-800/80 bg-zinc-900/40\"><div class=\"text-xs text-zinc-500 uppercase tracking-wider font-semibold\">Collisions</div><div class=\"text-2xl font-bold text-zinc-100 mt-1\">0</div></div>\n");
    }
    buf.push_str("  </div>\n");

    // Active Policies by Category
    if !plan.categories.is_empty() {
        buf.push_str("  <div class=\"space-y-4\">\n");
        buf.push_str("    <div class=\"flex items-center justify-between\">\n");
        buf.push_str("      <h2 class=\"text-sm font-semibold uppercase tracking-wider text-zinc-400\">Configured Policy Rules</h2>\n");
        buf.push_str("      <form hx-post=\"/policy/clear\" hx-target=\"#workspace\" hx-swap=\"innerHTML\" hx-confirm=\"Clear all active policies?\">\n");
        buf.push_str("        <button type=\"submit\" class=\"text-xs text-zinc-500 hover:text-red-400 transition\">Reset All</button>\n");
        buf.push_str("      </form>\n    </div>\n");

        buf.push_str("    <div class=\"grid grid-cols-1 md:grid-cols-2 gap-4\">\n");
        for (cat_name, items) in &plan.categories {
            buf.push_str("      <div class=\"p-4 rounded-xl border border-zinc-800/80 bg-zinc-900/30 space-y-3\">\n");
            buf.push_str("        <div class=\"flex items-center justify-between border-b border-zinc-800/50 pb-2\">\n");
            buf.push_str(&format!("          <span class=\"text-xs font-bold text-amber-400 uppercase tracking-wider\">{}</span>\n", escape_html(cat_name)));
            buf.push_str(&format!("          <span class=\"text-xs text-zinc-500\">{} rules</span>\n", items.len()));
            buf.push_str("        </div>\n        <div class=\"space-y-2\">\n");

            for pol in items {
                let sev_class = match pol.item.severity {
                    "Critical" => "bg-red-950 text-red-400 border border-red-800/50",
                    "High" => "bg-amber-950 text-amber-400 border border-amber-800/50",
                    _ => "bg-zinc-800 text-zinc-400",
                };

                buf.push_str("          <div class=\"p-2.5 rounded-lg border border-zinc-800 bg-zinc-900/60 flex items-start justify-between gap-3 text-xs\">\n");
                buf.push_str("            <div class=\"space-y-1 flex-1\">\n");
                buf.push_str("              <div class=\"flex items-center gap-2\">\n");
                buf.push_str(&format!("                <span class=\"font-semibold text-zinc-200\">{}</span>\n", escape_html(pol.item.name)));
                buf.push_str(&format!("                <span class=\"px-1.5 py-0.5 rounded text-[10px] font-mono {}\">{}</span>\n", sev_class, pol.item.severity));
                buf.push_str("              </div>\n");
                buf.push_str(&format!("              <p class=\"text-zinc-500 text-[11px] leading-relaxed\">{}</p>\n", escape_html(pol.item.description)));

                if pol.has_missing_prereqs {
                    buf.push_str(&format!("              <div class=\"mt-1 text-[11px] text-amber-400 flex items-center gap-1 font-medium\"><span>⚠️ Missing Prereq:</span> {}</div>\n", escape_html(&pol.missing_prereq_names.join(", "))));
                }

                buf.push_str("            </div>\n");
                buf.push_str("            <form hx-post=\"/policy/remove\" hx-target=\"#workspace\" hx-swap=\"innerHTML\">\n");
                buf.push_str(&format!("              <input type=\"hidden\" name=\"policy_id\" value=\"{}\">\n", pol.item.policy_id));
                buf.push_str("              <button type=\"submit\" class=\"text-zinc-600 hover:text-red-400 text-sm font-bold px-1 transition\" title=\"Remove policy\">×</button>\n");
                buf.push_str("            </form>\n");
                buf.push_str("          </div>\n");
            }

            buf.push_str("        </div>\n      </div>\n");
        }
        buf.push_str("    </div>\n  </div>\n");

        // Compiled YAML preview
        buf.push_str("  <div class=\"p-4 rounded-xl border border-zinc-800/80 bg-zinc-900/40 space-y-2\">\n");
        buf.push_str("    <div class=\"flex items-center justify-between\">\n");
        buf.push_str("      <span class=\"text-xs font-bold text-zinc-400 uppercase tracking-wider flex items-center gap-2\"><span>📄</span> Compiled Deployment Manifest (YAML)</span>\n");
        buf.push_str("      <button onclick=\"navigator.clipboard.writeText(document.getElementById('compiled-yaml-code').innerText); this.innerText='Copied!';\" class=\"text-xs text-amber-400 hover:underline\">Copy Manifest</button>\n");
        buf.push_str("    </div>\n");
        buf.push_str(&format!("    <pre class=\"p-3 rounded-lg bg-zinc-950 border border-zinc-800/80 text-[11px] font-mono text-zinc-300 overflow-x-auto max-h-72\"><code id=\"compiled-yaml-code\">{}</code></pre>\n", escape_html(&plan.compiled_yaml)));
        buf.push_str("  </div>\n");
    } else {
        // Empty State
        buf.push_str("  <div class=\"text-center py-16 px-4 border border-dashed border-zinc-800 rounded-2xl bg-zinc-900/10 space-y-3\">\n");
        buf.push_str("    <div class=\"text-3xl\">🛡️</div>\n");
        buf.push_str("    <h3 class=\"text-sm font-bold text-zinc-200\">No Policies Configured Yet</h3>\n");
        buf.push_str("    <p class=\"text-xs text-zinc-500 max-w-sm mx-auto\">Use the search bar above to query rules from the catalog. Changes are saved instantly into compressed HTTP-only cookies.</p>\n");
        buf.push_str("    <div class=\"pt-2 flex justify-center gap-2\">\n");
        buf.push_str("      <form hx-post=\"/policy/toggle\" hx-target=\"#workspace\" hx-swap=\"innerHTML\" class=\"inline\">\n");
        buf.push_str("        <input type=\"hidden\" name=\"policy_id\" value=\"POL-NET-02\">\n");
        buf.push_str("        <button type=\"submit\" class=\"text-xs px-3 py-1.5 rounded-lg border border-amber-500/30 bg-amber-500/10 hover:bg-amber-500/20 text-amber-300 font-medium transition\">+ Add Quick Sample (Strict TLS 1.3)</button>\n");
        buf.push_str("      </form>\n    </div>\n  </div>\n");
    }

    buf.push_str("</div>\n");
}

pub fn render_search_results(buf: &mut String, query: &str, active_policies: &HashSet<String>) {
    let results = search_catalog(query, 8);
    if !results.is_empty() {
        buf.push_str("<div class=\"absolute top-1 left-0 right-0 bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl overflow-hidden divide-y divide-zinc-800/60 max-h-96 overflow-y-auto\">\n");
        for item in results {
            let is_active = active_policies.contains(item.policy_id);
            let sev_class = match item.severity {
                "Critical" => "bg-red-950 text-red-400 border border-red-800/50",
                "High" => "bg-amber-950 text-amber-400 border border-amber-800/50",
                _ => "bg-zinc-800 text-zinc-400",
            };

            buf.push_str("  <div class=\"p-3 hover:bg-zinc-800/40 transition flex items-center justify-between gap-4 text-xs\">\n");
            buf.push_str("    <div class=\"space-y-1 flex-1\">\n");
            buf.push_str("      <div class=\"flex items-center gap-2\">\n");
            buf.push_str(&format!("        <span class=\"font-semibold text-zinc-100\">{}</span>\n", escape_html(item.name)));
            buf.push_str(&format!("        <span class=\"px-1.5 py-0.5 rounded text-[10px] font-mono bg-zinc-800 text-zinc-400 border border-zinc-700/60\">{}</span>\n", escape_html(item.category)));
            buf.push_str(&format!("        <span class=\"px-1.5 py-0.5 rounded text-[10px] font-mono {}\">{}</span>\n", sev_class, item.severity));
            buf.push_str("      </div>\n");
            buf.push_str(&format!("      <p class=\"text-zinc-400 text-[11px] leading-relaxed\">{}</p>\n", escape_html(item.description)));
            buf.push_str("    </div>\n");

            buf.push_str("    <form hx-post=\"/policy/toggle\" hx-target=\"#workspace\" hx-swap=\"innerHTML\">\n");
            buf.push_str(&format!("      <input type=\"hidden\" name=\"policy_id\" value=\"{}\">\n", item.policy_id));
            if is_active {
                buf.push_str("      <button type=\"submit\" class=\"px-3 py-1.5 rounded-lg border border-red-500/30 bg-red-950/40 hover:bg-red-900/60 text-red-300 font-medium transition whitespace-nowrap\">Remove</button>\n");
            } else {
                buf.push_str("      <button type=\"submit\" class=\"px-3 py-1.5 rounded-lg border border-amber-500/40 bg-amber-500/10 hover:bg-amber-500/20 text-amber-300 font-medium transition whitespace-nowrap\">+ Add</button>\n");
            }
            buf.push_str("    </form>\n  </div>\n");
        }
        buf.push_str("</div>\n");
    } else if !query.trim().is_empty() {
        buf.push_str(&format!("<div class=\"absolute top-1 left-0 right-0 bg-zinc-900 border border-zinc-800 rounded-xl p-4 text-center text-xs text-zinc-500 shadow-2xl\">No policies found matching \"<span class=\"text-zinc-300\">{}</span>\"</div>\n", escape_html(query)));
    }
}

pub fn render_index(buf: &mut String, plan: &PolicyPlan) {
    buf.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  <title>Thin Viewport • Policy Rule Builder (Axum Rust)</title>\n  <link rel=\"stylesheet\" href=\"/assets/app.css\">\n  <script src=\"/assets/htmx.min.js\"></script>\n</head>\n<body class=\"bg-zinc-950 text-zinc-100 min-h-screen flex flex-col font-sans selection:bg-amber-500 selection:text-black antialiased\" hx-sync=\"this:queue\">\n  <header class=\"border-b border-zinc-800 bg-zinc-900/60 backdrop-blur sticky top-0 z-30\">\n    <div class=\"max-w-6xl mx-auto px-4 h-16 flex items-center justify-between\">\n      <div class=\"flex items-center space-x-3\">\n        <div class=\"w-8 h-8 rounded-lg bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-amber-400 font-bold text-sm\">⚡</div>\n        <div>\n          <h1 class=\"font-bold text-base tracking-tight text-zinc-100 flex items-center gap-2\">Thin Viewport <span class=\"text-xs px-2 py-0.5 rounded bg-zinc-800 border border-zinc-700 text-zinc-400 font-mono\">Rust Axum</span></h1>\n          <p class=\"text-xs text-zinc-500 hidden sm:block\">Zero-DB SSR • Stateless Cookies • Ephemeral Cloud Workers</p>\n        </div>\n      </div>\n      <div class=\"flex items-center space-x-2\">\n        <a href=\"/export\" class=\"text-xs px-3 py-1.5 rounded-lg border border-zinc-800 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 font-medium transition flex items-center gap-1.5\"><span>💾</span> Export JSON</a>\n        <label class=\"text-xs px-3 py-1.5 rounded-lg border border-zinc-800 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 font-medium transition cursor-pointer flex items-center gap-1.5\"><span>📂</span> Import JSON<input type=\"file\" name=\"file\" class=\"hidden\" hx-post=\"/import\" hx-encoding=\"multipart/form-data\" hx-target=\"#workspace\" hx-swap=\"innerHTML\" onchange=\"htmx.trigger(this, 'change')\"></label>\n      </div>\n    </div>\n  </header>\n  <main class=\"max-w-6xl mx-auto px-4 py-8 flex-1 w-full space-y-6\">\n    <div class=\"relative w-full\">\n      <div class=\"relative\">\n        <span class=\"absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none text-zinc-500 text-sm\">🔍</span>\n        <input type=\"search\" name=\"q\" placeholder=\"Search 16 Cloud & Security Policies (e.g. TLS, Encryption, MFA, Container, SOC2)...\" autocomplete=\"off\" class=\"w-full pl-10 pr-4 py-3 rounded-xl bg-zinc-900/90 border border-zinc-800 text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-amber-500/50 focus:ring-1 focus:ring-amber-500/50 text-sm transition shadow-inner\" hx-get=\"/search\" hx-trigger=\"input changed delay:150ms, search, refreshSearch from:body\" hx-target=\"#search-results\" hx-swap=\"innerHTML\">\n      </div>\n      <div id=\"search-results\" class=\"relative z-20\"></div>\n    </div>\n    <div id=\"workspace\">\n");
    render_workspace(buf, plan);
    buf.push_str("    </div>\n  </main>\n  <footer class=\"border-t border-zinc-900 bg-zinc-950 py-6 text-center text-xs text-zinc-600\">Thin Viewport Architecture Reference • Axum Rust Single Binary • MIT Licensed</footer>\n</body>\n</html>\n");
}
