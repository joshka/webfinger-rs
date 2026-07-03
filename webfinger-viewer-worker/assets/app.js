// Small browser-only behavior for the viewer shell.
//
// htmx owns lookup submission and result swapping. Keep this file limited to behavior that is local
// to the browser: theme persistence, path-prefix-aware API targeting, loading state, and clipboard
// buttons. Parsed JRD rendering belongs in Rust view models and Askama templates.

const state = document.querySelector("#state");
const form = document.querySelector("#lookup-form");
const resourceInput = document.querySelector("#resource");
const submit = document.querySelector("#submit");
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
// escape that mount. htmx reads this attribute when the form submits, so setting it once during page
// initialization keeps the template free of deployment-specific path logic.
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

form.setAttribute("hx-get", apiPath());
resourceInput.setAttribute("placeholder", resourcePlaceholder());

// While htmx owns the request, the surrounding shell owns the global loading indicator.
//
// Target WebFinger status is rendered by the response fragment after the request completes. This
// transient state only tells the user that the Worker lookup is in flight.
document.body.addEventListener("htmx:beforeRequest", () => {
  submit.disabled = true;
  state.textContent = "Fetching";
  state.className = "state-text warn";
});

document.body.addEventListener("htmx:afterRequest", () => {
  submit.disabled = false;
});

themeSelect.addEventListener("change", () => applyTheme(themeSelect.value));

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
