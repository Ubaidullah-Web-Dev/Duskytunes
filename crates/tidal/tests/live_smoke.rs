//! Live TIDAL checks. Not in the default run, because they need a real signed-in account.

use duskytunes_tidal::{Entitlement, Tidal, TidalAuth};

/// The profile name is the one thing the token response never carries, so the only honest check
/// is against a real account. A `None` is a pass too: it means the account never made a profile,
/// which is the common case, and the status line then shows the username.
///
///   DUSKYTUNES_TIDAL_TOKEN=… DUSKYTUNES_TIDAL_USER=… \
///     cargo test -p duskytunes-tidal --test live_smoke -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn the_profile_name_comes_off_the_account() {
    let (Ok(token), Ok(user)) =
        (std::env::var("DUSKYTUNES_TIDAL_TOKEN"), std::env::var("DUSKYTUNES_TIDAL_USER"))
    else {
        eprintln!("set DUSKYTUNES_TIDAL_TOKEN and DUSKYTUNES_TIDAL_USER to run this");
        return;
    };
    let t = Tidal::new(std::env::temp_dir().join("duskytunes-tidal-live"));
    t.set_auth(Some(TidalAuth {
        access_token: token,
        refresh_token: None,
        expires_at: i64::MAX,
        user_id: user.parse().expect("DUSKYTUNES_TIDAL_USER is the numeric account id"),
        country_code: std::env::var("DUSKYTUNES_TIDAL_COUNTRY").unwrap_or_else(|_| "DE".into()),
        entitlement: Entitlement::Standard,
        username: None,
        profile_name: None,
    }));

    let auth = t.load_profile_name().await.expect("the account endpoint answered");
    eprintln!("profile_name = {:?}", auth.profile_name);
    assert_eq!(auth.profile_name, t.auth().unwrap().profile_name, "the read is kept, not dropped");
    assert!(
        auth.profile_name.as_deref() != Some(""),
        "an unset profile name must arrive as None, never as an empty string the UI would print"
    );
}
