//! Login screen — port of `docs/open4x-landing/project/hifi/login.jsx`.
//!
//! Three stacked auth panels (Email · OpenID · atproto). Inputs are
//! present but no actual auth flow is wired — the buttons no-op until
//! `open4x-accounts` and the lobby's HTTP surface land (Phases 2-3 in
//! `book/src/roadmap/accounts-and-login.md`). The underlined help
//! triggers are now real `<Popup>` wrappers.

use std::sync::Arc;

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::api::auth as auth_api;
use crate::components::crypto;
use crate::components::i18n::{Key, tr};
use crate::components::{Btn, Popup, PopupBody, PopupSize};

/// State machine for the Ed25519 pubkey sign-in panel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum KeyFlow {
    #[default]
    Idle,
    Signing,
    Error(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum EmailFlow {
    #[default]
    Idle,
    Pending,
    Sent(String),
    Error(EmailFlowError),
}

/// Discriminated transient errors so the UI can pick the right copy
/// and decide whether a Retry affordance makes sense (Retry doesn't
/// help an empty-input validation, but it does help a 5xx).
#[derive(Clone, Debug, PartialEq, Eq)]
enum EmailFlowError {
    /// Local validation — empty input. No retry button; user just
    /// fills the field and clicks Send.
    EmptyEmail,
    /// 429 rate-limited (per-email or per-IP throttle). Includes
    /// the server's hint message so "Retry-After" copy is honest.
    RateLimited { message: Option<String> },
    /// 5xx — mint or mailer failed. Worth retrying after a moment.
    ServerBusy {
        code: String,
        message: Option<String>,
    },
    /// Network / transport failure — couldn't reach the server.
    /// `ApiError::status == 0` from the binding.
    Network { detail: String },
    /// Anything else (4xx that isn't 429, unrecognised codes).
    Other {
        code: String,
        message: Option<String>,
    },
}

impl EmailFlowError {
    fn from_api(e: &crate::components::api::ApiError) -> Self {
        let msg = e.message.clone();
        match e.status {
            0 => EmailFlowError::Network {
                detail: msg
                    .clone()
                    .unwrap_or_else(|| "couldn't reach the server".into()),
            },
            429 => EmailFlowError::RateLimited { message: msg },
            500..=599 => EmailFlowError::ServerBusy {
                code: e.code.clone(),
                message: msg,
            },
            _ => EmailFlowError::Other {
                code: e.code.clone(),
                message: msg,
            },
        }
    }

    fn is_retryable(&self) -> bool {
        // EmptyEmail isn't retryable (the user has to act); everything
        // else benefits from a Retry button.
        !matches!(self, EmailFlowError::EmptyEmail)
    }

    /// Human copy for the inline error line. Optimistic where the
    /// transient kinds are concerned.
    fn copy(&self) -> String {
        match self {
            EmailFlowError::EmptyEmail => tr(Key::LoginEmailErrorEmpty).into(),
            EmailFlowError::RateLimited { message } => message
                .clone()
                .unwrap_or_else(|| tr(Key::LoginEmailErrorRateLimit).into()),
            EmailFlowError::ServerBusy { code, message } => match message {
                Some(m) => format!("Server's having trouble ({code}): {m}"),
                None => tr(Key::LoginEmailErrorServerBusyTemplate).replace("{code}", code),
            },
            EmailFlowError::Network { detail } => {
                format!("{}\n{detail}", tr(Key::LoginEmailErrorNetwork))
            }
            EmailFlowError::Other { code, message } => match message {
                Some(m) => format!("{code}: {m}"),
                None => code.clone(),
            },
        }
    }
}

#[component]
pub fn Login(
    on_back: Callback<()>,
    /// Fired when an interactive sign-in (currently pubkey) completes and
    /// the session cookie is set — the app shell switches to the menu.
    on_authenticated: Callback<()>,
) -> impl IntoView {
    let email = RwSignal::new(String::new());
    let flow = RwSignal::new(EmailFlow::Idle);
    let key_flow = RwSignal::new(KeyFlow::Idle);

    // Pubkey challenge-response sign-in. Loads (or generates) the
    // browser's Ed25519 key, signs the server nonce, and verifies. The
    // email field, when filled, is linked as a secondary identity.
    let do_pubkey = move || {
        key_flow.set(KeyFlow::Signing);
        let email_opt = {
            let e = email.get_untracked().trim().to_string();
            if e.is_empty() { None } else { Some(e) }
        };
        spawn_local(async move {
            let sk = match crypto::load_or_create_key() {
                Ok(k) => k,
                Err(e) => return key_flow.set(KeyFlow::Error(e)),
            };
            let pubkey = crypto::public_key_hex(&sk);
            let challenge = match auth_api::pubkey_challenge(pubkey.clone()).await {
                Ok(c) => c,
                Err(e) => return key_flow.set(KeyFlow::Error(e.to_string())),
            };
            let signature = match crypto::sign_challenge(&sk, &challenge.nonce) {
                Ok(s) => s,
                Err(e) => return key_flow.set(KeyFlow::Error(e)),
            };
            match auth_api::pubkey_verify(pubkey, signature, email_opt).await {
                Ok(_) => {
                    key_flow.set(KeyFlow::Idle);
                    on_authenticated.run(());
                }
                Err(e) => key_flow.set(KeyFlow::Error(e.to_string())),
            }
        });
    };
    let on_pubkey = move |_| do_pubkey();
    let key_signing = Signal::derive(move || matches!(key_flow.get(), KeyFlow::Signing));

    let do_send = move || {
        let addr = email.get_untracked().trim().to_string();
        if addr.is_empty() {
            flow.set(EmailFlow::Error(EmailFlowError::EmptyEmail));
            return;
        }
        flow.set(EmailFlow::Pending);
        spawn_local(async move {
            match auth_api::email_start(addr.clone()).await {
                Ok(_) => flow.set(EmailFlow::Sent(addr)),
                Err(e) => flow.set(EmailFlow::Error(EmailFlowError::from_api(&e))),
            }
        });
    };
    let on_send = move |_| do_send();
    let on_retry = move |_| do_send();

    let pending = Signal::derive(move || matches!(flow.get(), EmailFlow::Pending));
    view! {
        <div style="flex:1; display:flex; flex-direction:column">
            <div class="row between center-y" style="padding:10px 20px; border-bottom:1px solid var(--hairline)">
                <Btn variant="bare" size="sm"
                     on_click=Callback::new(move |_| on_back.run(()))>
                    {tr(Key::LoginBack)}
                </Btn>
                <span class="muted xsmall" style="letter-spacing:0.08em">
                    {tr(Key::LoginNavTitle)}
                </span>
            </div>

            <div style="width:460px; max-width:100%; margin:32px auto; padding:4px">
                <h1 class="h1" style="margin-bottom:4px">{tr(Key::LoginPageTitle)}</h1>
                <p class="muted small" style="margin-bottom:24px">
                    {tr(Key::LoginIntroPrefix)}
                    <Popup
                        title="player ID"
                        content=Arc::new(|| view! {
                            <PopupBody>
                                <p>"An opaque, unique identifier (e.g. "
                                    <code>"0xA9C3·7F12·EE04"</code>
                                    ") created the first time you sign in."
                                </p>
                                <p>"All three login methods can be linked to the same player ID, so you can reach your games from any device."</p>
                                <p class="muted xsmall">"There are no usernames. Friends find you by your linked email, OpenID URL, or atproto handle."</p>
                            </PopupBody>
                        }.into_any())
                    >
                        <span class="trigger">{tr(Key::LoginIntroPlayerIdLink)}</span>
                    </Popup>
                    {tr(Key::LoginIntroSuffix)}
                </p>

                // ─── Email ──────────────────────────────────────────
                <div class="panel" style="margin-bottom:12px">
                    <div class="row between center-y" style="margin-bottom:10px">
                        <span class="h3">{tr(Key::LoginEmailHeading)}</span>
                        <Popup
                            title="How it works"
                            size=PopupSize::Narrow
                            content=Arc::new(|| view! {
                                <PopupBody>
                                    <p>"1. Enter your email."</p>
                                    <p>"2. Receive a one-time magic link (valid 15 min)."</p>
                                    <p>"3. Click it — you're signed in."</p>
                                </PopupBody>
                            }.into_any())
                        >
                            <span class="trigger xsmall muted">{tr(Key::LoginEmailHowItWorks)}</span>
                        </Popup>
                    </div>
                    <div class="field" style="margin-bottom:10px">
                        <input
                            class="input mono"
                            placeholder=tr(Key::LoginEmailPlaceholder)
                            prop:value=move || email.get()
                            on:input=move |ev| {
                                use wasm_bindgen::JsCast as _;
                                if let Some(el) = ev
                                    .target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                {
                                    email.set(el.value());
                                }
                            }
                        />
                    </div>
                    <Btn
                        variant="primary"
                        class="block"
                        disabled=pending
                        on_click=Callback::new(on_send)
                    >
                        {move || if pending.get() {
                            tr(Key::LoginEmailSendingButton)
                        } else {
                            tr(Key::LoginEmailSendButton)
                        }}
                    </Btn>
                    {move || match flow.get() {
                        EmailFlow::Sent(to) => view! {
                            <p class="xsmall" style="color:var(--good); margin-top:8px">
                                {tr(Key::LoginEmailSentTemplate).replace("{to}", &to)}
                            </p>
                        }.into_any(),
                        EmailFlow::Error(err) => {
                            let copy = err.copy();
                            let retryable = err.is_retryable();
                            view! {
                                <div style="margin-top:8px">
                                    <p class="xsmall" style="color:var(--accent); white-space:pre-line">
                                        {copy}
                                    </p>
                                    {retryable.then(|| view! {
                                        <Btn
                                            variant="ghost"
                                            size="sm"
                                            on_click=Callback::new(on_retry)
                                        >{tr(Key::LoginEmailRetry)}</Btn>
                                    })}
                                </div>
                            }.into_any()
                        }
                        _ => view! { <span /> }.into_any(),
                    }}
                </div>

                // ─── Passkey (Ed25519) ──────────────────────────────
                <div class="panel" style="margin-bottom:12px">
                    <div class="row between center-y" style="margin-bottom:10px">
                        <span class="h3">"Sign in with a key"</span>
                        <Popup
                            title="Key sign-in"
                            size=PopupSize::Narrow
                            content=Arc::new(|| view! {
                                <PopupBody>
                                    <p>"Your browser holds an Ed25519 keypair (created on first use). Signing in proves you hold the private key — no password, no email round-trip."</p>
                                    <p class="muted xsmall">"Fill the email above first to link it as a recovery identity. The key is stored in this browser only."</p>
                                </PopupBody>
                            }.into_any())
                        >
                            <span class="trigger xsmall muted">"how it works"</span>
                        </Popup>
                    </div>
                    <Btn
                        variant="primary"
                        class="block"
                        disabled=key_signing
                        on_click=Callback::new(on_pubkey)
                    >
                        {move || if key_signing.get() { "Signing…" } else { "Sign in with key" }}
                    </Btn>
                    {move || match key_flow.get() {
                        KeyFlow::Error(msg) => view! {
                            <p class="xsmall" style="color:var(--accent); margin-top:8px; white-space:pre-line">
                                {msg}
                            </p>
                        }.into_any(),
                        _ => view! { <span /> }.into_any(),
                    }}
                </div>

                // ─── OpenID ─────────────────────────────────────────
                <div class="panel" style="margin-bottom:12px">
                    <div class="row between center-y" style="margin-bottom:10px">
                        <span class="h3">{tr(Key::LoginOidcHeading)}</span>
                        <Popup
                            title="OIDC"
                            content=Arc::new(|| view! {
                                <PopupBody>
                                    <p>"Standard OpenID Connect flow. We support common providers and any custom issuer URL."</p>
                                    <p class="muted xsmall">"Tokens are stored client-side; the server only sees signed claims."</p>
                                </PopupBody>
                            }.into_any())
                        >
                            <span class="trigger xsmall muted">{tr(Key::LoginOidcAbout)}</span>
                        </Popup>
                    </div>
                    <div class="row wrap" style="gap:6px">
                        {["Google", "GitHub", "GitLab", "Microsoft"].iter().map(|p| view! {
                            <Btn size="sm">{*p}</Btn>
                        }).collect::<Vec<_>>()}
                        <Btn variant="ghost" size="sm" class="has-popup">"Custom OIDC…"</Btn>
                    </div>
                </div>

                // ─── atproto ────────────────────────────────────────
                <div class="panel" style="margin-bottom:16px">
                    <div class="row between center-y" style="margin-bottom:10px">
                        <span class="h3">{tr(Key::LoginAtprotoHeading)}</span>
                        <Popup
                            title="atproto"
                            content=Arc::new(|| view! {
                                <PopupBody>
                                    <p>"Use your handle (e.g. "
                                        <code>"alice.bsky.social"</code>
                                        ") or DID. We resolve your PDS and start an OAuth flow."
                                    </p>
                                    <p class="muted xsmall">"Works with self-hosted PDSes too."</p>
                                </PopupBody>
                            }.into_any())
                        >
                            <span class="trigger xsmall muted">{tr(Key::LoginAtprotoAbout)}</span>
                        </Popup>
                    </div>
                    <div class="field" style="margin-bottom:10px">
                        <input class="input mono"
                               placeholder=tr(Key::LoginAtprotoPlaceholder) />
                    </div>
                    <Btn class="block">{tr(Key::LoginAtprotoButton)}</Btn>
                </div>

                <p class="muted xsmall" style="text-align:center; margin-top:14px">
                    {tr(Key::LoginFooter)}
                </p>
            </div>
        </div>
    }
}
