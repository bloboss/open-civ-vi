//! Lobby i18n shim — Phase 5 polish.
//!
//! The lobby's user-facing strings are hard-coded English literals
//! today. This module is the seam: replace `"Sign in"` literals with
//! `tr(Key::SigninTitle)` and the SPA stays English-only at runtime
//! while the surface-area for adding a second language collapses to
//! "extend [`Key`] + add a match arm in [`tr`]".
//!
//! Deliberately *not* a library. Reasons:
//! 1. We're adding a few-dozen strings, not a few-thousand.
//! 2. fluent / gettext / cargo-i18n add 10s of MB of wasm and a
//!    runtime parser. A `match` statement is byte-for-byte cheaper
//!    and the compiler enforces exhaustiveness.
//! 3. Most candidates either bundle every locale into the wasm or
//!    require an XHR per language; both are worse for a lobby
//!    that loads on a cold cache.
//!
//! When a second language ships:
//! 1. Add a variant to [`Locale`].
//! 2. Provide a `Locale` value via `provide_context` at App root
//!    (e.g. seeded from `navigator.language` or a user pref).
//! 3. Update [`tr_in`] to dispatch on the locale.
//! 4. Existing call sites stay as-is — they all funnel through
//!    [`tr`].

#![cfg(feature = "csr")]

/// Languages the lobby can render in. Only `En` is wired today;
/// adding a variant is the first step towards a translated build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    En,
}

/// Every translatable string the SPA renders, named so the
/// compiler enforces exhaustiveness. Keys are grouped by the
/// screen that owns them; convention is `<screen>_<role>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // Login
    LoginBack,
    LoginNavTitle,
    LoginPageTitle,
    LoginIntroPlayerIdLink,
    LoginIntroPrefix,
    LoginIntroSuffix,
    LoginEmailHeading,
    LoginEmailHowItWorks,
    LoginEmailPlaceholder,
    LoginEmailSendButton,
    LoginEmailSendingButton,
    LoginEmailSentTemplate,
    LoginEmailRetry,
    LoginEmailErrorEmpty,
    LoginEmailErrorRateLimit,
    LoginEmailErrorServerBusyTemplate,
    LoginEmailErrorNetwork,
    LoginEmailErrorOther,
    LoginOidcHeading,
    LoginOidcAbout,
    LoginAtprotoHeading,
    LoginAtprotoAbout,
    LoginAtprotoPlaceholder,
    LoginAtprotoButton,
    LoginFooter,
}

/// Look up `key` in `locale`. Today every locale is `En`, so the
/// outer match is degenerate; the structure is what we'll grow
/// into when a second language ships.
pub fn tr_in(locale: Locale, key: Key) -> &'static str {
    match locale {
        Locale::En => en(key),
    }
}

/// Convenience: look up `key` in the default locale (`En` today).
/// Most callers want this; the few that need to react to a
/// runtime locale change should route through [`tr_in`] from
/// inside a `move ||` closure that reads the locale from context.
pub fn tr(key: Key) -> &'static str {
    tr_in(Locale::default(), key)
}

fn en(key: Key) -> &'static str {
    use Key::*;
    match key {
        LoginBack => "← back",
        LoginNavTitle => "OPEN4X·VI / SIGN IN",
        LoginPageTitle => "Sign in",
        LoginIntroPlayerIdLink => "player ID",
        LoginIntroPrefix => "Any method below — they all link to the same ",
        LoginIntroSuffix => ".",
        LoginEmailHeading => "Email",
        LoginEmailHowItWorks => "how it works",
        LoginEmailPlaceholder => "you@example.com",
        LoginEmailSendButton => "Send magic link →",
        LoginEmailSendingButton => "Sending…",
        LoginEmailSentTemplate => "Magic link sent to {to}. Check your inbox.",
        LoginEmailRetry => "↻ Try again",
        LoginEmailErrorEmpty => "Enter an email address first.",
        LoginEmailErrorRateLimit => "Too many recent sends. Try again in a minute.",
        LoginEmailErrorServerBusyTemplate => {
            "Server's having trouble ({code}). Try again in a moment."
        }
        LoginEmailErrorNetwork => "Couldn't reach the server. Check your connection.",
        LoginEmailErrorOther => "Something went wrong.",
        LoginOidcHeading => "OpenID",
        LoginOidcAbout => "about OIDC",
        LoginAtprotoHeading => "atproto",
        LoginAtprotoAbout => "about atproto",
        LoginAtprotoPlaceholder => "alice.bsky.social  or  did:plc:…",
        LoginAtprotoButton => "Continue with atproto →",
        LoginFooter => "New here? A player ID is created automatically on first sign-in.",
    }
}

// Compiler-enforced exhaustiveness on the enum match in `en()`
// is the test: a missing variant fails to compile. No runtime
// test module — the csr-only build target can't run unit tests
// natively (wasm32-unknown-unknown).
