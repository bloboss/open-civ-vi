//! `end-turn` over REST: `POST /turn/end`. Refreshes the session
//! file's cached turn number so the next invocation has a current
//! reference even before it issues another request.

use std::path::Path;

use serde_json::json;

use super::client::ApiClient;
use super::session::Session;

pub fn end_turn(server: &str, token_file: &Path) -> Result<(), String> {
    let mut session = Session::load(token_file)?;
    let client = ApiClient::new(server).with_token(session.token.clone());

    let resp = client.post_json("/api/v1/turn/end", &json!({}))?;

    if let Some(t) = resp
        .get("turn_status")
        .and_then(|v| v.get("turn"))
        .and_then(|v| v.as_u64())
    {
        session.turn = t as u32;
        let _ = session.save(token_file);
    }

    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    Ok(())
}
