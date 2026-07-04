// Small browser-only behavior for the viewer shell.
//
// htmx owns lookup submission and result swapping. Keep this file limited to behavior that is local
// to the browser: theme persistence, path-prefix-aware API targeting, context-sensitive
// placeholders, and clipboard buttons. Parsed JRD rendering, lookup lifecycle state, and initial
// form hydration belong to Rust view models, Askama templates, and htmx attributes.

const form = document.querySelector("#lookup-form");
const resourceInput = document.querySelector("#resource");
const relInput = document.querySelector("#rels");
const relPresetInputs = Array.from(document.querySelectorAll(".rel-preset input[name='rel']"));
const themeSelect = document.querySelector("#theme");
const themeStorageKey = "webfinger-viewer-theme";

// Applies a user-selected theme while keeping `auto` mapped to native prefers-color-scheme.
//
// The page should be usable when embedded under any path and without a server round trip for
// preferences. Validate theme changes by switching all three options and reloading the page.
function applyTheme(choice) {
  const normalized = ["auto", "light", "dark"].includes(choice) ? choice : "auto";
  if (normalized === "auto") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.dataset.theme = normalized;
  }
  themeSelect.value = normalized;
  localStorage.setItem(themeStorageKey, normalized);
}

applyTheme(localStorage.getItem(themeStorageKey) || "auto");

// Computes the lookup endpoint below the current page path.
//
// This Worker is intended to mount at paths like `/webfinger`; hard-coding `/api/lookup` would
// escape that mount. htmx reads `hx-post` when the form submits, so setting it once during page
// initialization keeps the template free of deployment-specific path logic while preserving the
// POST-only rule for outbound WebFinger fetches.
function apiPath() {
  const base = window.location.pathname.replace(/\/$/, "");
  return `${base || ""}/api/lookup`;
}

// Shows the most useful example for the current deployment context.
//
// Public deployments are same-origin by default, so a host-specific acct URI is the right first
// hint. Local Wrangler sessions are the exception: the common debugging task is pointing this
// viewer at another local Worker, usually by pasting a full WebFinger URL with that server's port.
function resourcePlaceholder() {
  const host = window.location.hostname;
  if (["localhost", "127.0.0.1", "::1"].includes(host)) {
    return "http://127.0.0.1:8787/.well-known/webfinger?resource=acct%3Aalice%40localhost";
  }
  return `acct:alice@${host}`;
}

// Returns URL search parameters for either an htmx event path or the current location.
//
// htmx history events provide `detail.path`, which is the path/query being saved or restored.
// Prefer that over `location.search`: during POST submissions and history traversal, the address
// bar can lag the snapshot that htmx is currently processing.
function searchParamsForPath(path) {
  return new URL(path || window.location.href, window.location.origin).searchParams;
}

// Applies a viewer URL query to the form without issuing a lookup.
//
// Server-side Askama rendering handles hard refreshes. This browser-side copy exists for htmx
// history restores: htmx can restore cached results locally, but the restored form controls may
// contain values typed for the next lookup because inputs store live state as properties.
function hydrateFormFromPath(path) {
  const params = searchParamsForPath(path);
  resourceInput.value = params.get("resource") || "";

  const rels = params.getAll("rel").flatMap((value) =>
    value
      .split(/[,\n]/)
      .map((rel) => rel.trim())
      .filter(Boolean),
  );
  const selectedRels = new Set(rels);
  const presetValues = new Set(relPresetInputs.map((input) => input.value));

  for (const input of relPresetInputs) {
    input.checked = selectedRels.has(input.value);
  }
  relInput.value = rels.filter((rel) => !presetValues.has(rel)).join(", ");
}

// Applies the current viewer URL query to the form without issuing a lookup.
function hydrateFormFromLocation() {
  hydrateFormFromPath(window.location.href);
}

// Rehydrates after browser history traversal.
//
// The zero-delay pass handles ordinary browser restoration. The second pass handles htmx snapshot
// restoration that settles just after the history event; both passes are idempotent and never fetch.
function hydrateFormAfterHistoryRestore(event) {
  const path = event?.detail?.path || window.location.href;
  window.setTimeout(() => hydrateFormFromPath(path), 0);
  window.setTimeout(() => hydrateFormFromPath(path), 50);
}

// Copies a value when the Clipboard API is available.
//
// Missing text is a no-op because some controls copy optional output that is absent before the
// first lookup. The UI intentionally does not show a toast; hover/focus affordances identify the
// copy target without adding more status state to the debugging flow.
async function copy(text) {
  if (text) {
    await navigator.clipboard.writeText(text);
  }
}

// Persists the URL-represented form state into attributes before htmx snapshots history.
//
// htmx snapshots mutable form controls into its history cache. Use `detail.path`, not live input
// properties, so each cached form reflects the URL htmx is saving even when the user has already
// typed the next POST body.
function syncFormStateForHistory(event) {
  const params = searchParamsForPath(event?.detail?.path);
  resourceInput.setAttribute("value", params.get("resource") || "");

  const rels = params.getAll("rel").flatMap((value) =>
    value
      .split(/[,\n]/)
      .map((rel) => rel.trim())
      .filter(Boolean),
  );
  const selectedRels = new Set(rels);
  const presetValues = new Set(relPresetInputs.map((input) => input.value));
  relInput.setAttribute("value", rels.filter((rel) => !presetValues.has(rel)).join(", "));
  for (const input of relPresetInputs) {
    if (selectedRels.has(input.value)) {
      input.setAttribute("checked", "");
    } else {
      input.removeAttribute("checked");
    }
  }
}

form.setAttribute("hx-post", apiPath());
resourceInput.setAttribute("placeholder", resourcePlaceholder());

themeSelect.addEventListener("change", () => applyTheme(themeSelect.value));
document.body.addEventListener("htmx:beforeHistorySave", syncFormStateForHistory);
document.body.addEventListener("htmx:historyRestore", hydrateFormAfterHistoryRestore);
window.addEventListener("popstate", hydrateFormAfterHistoryRestore);

// Use delegated copy handling because htmx replaces the result fragment after every lookup.
//
// Binding to `document` means copy buttons inside newly swapped fragments work without re-running
// setup code. `data-copy` stores direct values; `data-copy-from` points at larger blocks such as
// the curl command where the visible text is the source of truth.
document.addEventListener("click", (event) => {
  const button = event.target.closest("[data-copy]");
  if (button) {
    copy(button.dataset.copy);
    return;
  }
  const copyFrom = event.target.closest("[data-copy-from]");
  if (copyFrom) {
    const source = document.querySelector(`#${copyFrom.dataset.copyFrom}`);
    copy(source?.textContent || "");
  }
});
