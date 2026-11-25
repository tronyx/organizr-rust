//! The invites plugin: codes that give a person access to the media server.
//!
//! A member makes an invite for a person. That person enters the code of the invite
//! with the name of the own Plex account, and Organizr shares the Plex server with the
//! account.

use std::time::{Duration, Instant, SystemTime};

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use organizr_auth::login_limit::{Attempt, LimitPolicy};
use organizr_db::DbError;
use organizr_domain::email::{self, php_trim};
use organizr_domain::{GroupLevel, Invite, NewInvite};
use organizr_integrations::plex::{PlexError, PlexTv, Share};
use organizr_plugins::mail::{MailKind, Message, Recipient};
use organizr_settings::Settings;
use serde::Serialize;

use crate::api_data::ApiData;
use crate::caller::{Caller, now_stamp};
use crate::client::Client;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::choice::Choice;
use crate::handlers::fields::Refusal;
use crate::handlers::mail::{self, Letter};
use crate::state::AppState;

/// The level that sees and deletes the invites of every member.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// What wrong codes are counted under, next to the address of the client. It holds a
/// character that no name of a user can hold.
const LIMIT_KEY: &str = "\0invite code";

/// The most characters of a code.
const CODE_LIMIT: usize = 50;

/// The most characters of a name or an e-mail address.
const TEXT_LIMIT: usize = 255;

/// An invite as the API shows it, with the field names of the PHP application.
#[derive(Debug, Serialize)]
pub struct InviteRow {
    id: i64,
    code: String,
    date: String,
    email: String,
    username: String,
    dateused: Option<String>,
    usedby: Option<String>,
    ip: Option<String>,
    /// `Yes` while the code can be used, then `No`.
    valid: &'static str,
    #[serde(rename = "type")]
    kind: String,
    invitedby: String,
}

impl From<Invite> for InviteRow {
    fn from(invite: Invite) -> Self {
        Self {
            id: invite.id,
            code: invite.code,
            date: date_and_time(invite.created_at),
            email: invite.email,
            username: invite.username,
            dateused: invite.used_at.map(date_and_time),
            usedby: invite.used_by,
            ip: invite.used_from,
            valid: if invite.is_valid { "Yes" } else { "No" },
            kind: invite.kind,
            invitedby: invite.invited_by,
        }
    }
}

/// A time as the PHP application stores it: `2026-11-18 19:02:06`, in UTC.
pub(crate) fn date_and_time(seconds: i64) -> String {
    jiff::Timestamp::from_second(seconds).map_or_else(
        |_| String::new(),
        |time| time.strftime("%Y-%m-%d %H:%M:%S").to_string(),
    )
}

/// The plugin answers only when it is switched on.
pub(crate) fn check_enabled(state: &AppState) -> Result<(), Refusal> {
    if state.settings().invites_enabled {
        Ok(())
    } else {
        Err(Refusal::conflict("INVITES-enabled module is not enabled"))
    }
}

/// The checks of the routes for the members who make invites.
fn check_member(state: &AppState, caller: &Caller) -> Result<(), Refusal> {
    check_enabled(state)?;
    let level = u16::try_from(state.settings().invites_auth_include).unwrap_or(0);
    caller.require(GroupLevel::new(level).unwrap_or(GroupLevel::ADMIN))?;
    Ok(())
}

fn is_manager(caller: &Caller) -> bool {
    caller.level() <= MANAGER
}

/// The user name of the caller. A caller with the API key has none.
fn name_of(caller: &Caller) -> String {
    caller
        .user()
        .map_or_default(|user| user.authenticated.user.username.clone())
}

/// `GET /plugins/invites`: every invite for an Admin or a Co-Admin, and the own
/// invites for each other member.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<InviteRow>>, Refusal> {
    check_member(&state, &caller)?;
    let own = (!is_manager(&caller)).then(|| name_of(&caller));
    let invites = state.database().invites().list(own.as_deref()).await?;
    Ok(Envelope::success(
        invites.into_iter().map(InviteRow::from).collect(),
    ))
}

/// Whether a text can be the code of an invite: letters, digits, `-` and `_`. A code
/// is a part of an address, so it holds nothing else.
fn is_code(text: &str) -> bool {
    !text.is_empty()
        && text.chars().count() <= CODE_LIMIT
        && text
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == '-' || letter == '_')
}

/// `POST /plugins/invites`: makes an invite. Fields of the body: `code`, `username`
/// and `email`. When the mailer is on, the invited person gets an e-mail with the code.
pub async fn create(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    check_member(&state, &caller)?;
    let settings = state.settings();
    let invites = state.database().invites();
    let maker = name_of(&caller);
    let most = u64::from(settings.invites_maximum_invites);
    if !is_manager(&caller) && most != 0 && invites.count_of(&maker).await? >= most {
        return Err(Refusal::conflict("Maximum number of invites reached"));
    }

    // What PHP takes for "nothing": no value, the empty text, and the text `0`.
    let field = |name: &str, message: &str| {
        data.text(name)
            .map(|value| php_trim(&value).to_owned())
            .filter(|value| !value.is_empty() && value != "0")
            .ok_or_else(|| Refusal::conflict(message))
    };
    let code = field("code", "Code not supplied")?;
    let username = field("username", "Username not supplied")?;
    let address = field("email", "Email not supplied")?;
    if !is_code(&code) {
        return Err(Refusal::conflict("Code is not valid"));
    }
    if username.chars().count() > TEXT_LIMIT || address.chars().count() > TEXT_LIMIT {
        return Err(Refusal::over_limit(TEXT_LIMIT));
    }
    if !email::is_valid(&address) {
        return Err(Refusal::invalid_email(&address));
    }

    let new = NewInvite {
        code: code.clone(),
        created_at: now_stamp(SystemTime::now()).unix_seconds(),
        email: address.clone(),
        username: username.clone(),
        kind: settings.invites_type_include.clone(),
        invited_by: maker,
    };
    match invites.insert(new).await {
        Ok(_) => {}
        Err(DbError::Duplicate) => {
            return Err(Refusal::conflict(format!(
                "Invite Code: {code} already exists"
            )));
        }
        Err(error) => return Err(error.into()),
    }
    tracing::info!(client = %client.address, "an invite was added");

    if settings.mailer_enabled {
        let site = mail::site_address(&settings, &headers, &client);
        let letter = Letter {
            kind: MailKind::Invite,
            subject: &settings.mailer_email_template_invite_user_subject,
            body: &settings.mailer_email_template_invite_user,
            user: Some(&username),
            password: None,
            invite_code: Some(&code),
        };
        let (subject, html) = mail::compose(&settings, &site, letter);
        let message = Message {
            to: Some(Recipient {
                address,
                name: Some(username),
            }),
            bcc: Vec::new(),
            subject,
            html,
        };
        mail::send_in_background(&state, message, "invite");
    }
    Ok(Envelope::success(NoData).with_message(format!("Invite Code: {code} has been created")))
}

/// The invite of a code that can still be used. A wrong code counts like a failed
/// login: by the address of the client. When the count reaches the `loginAttempts`
/// setting, the answer is `403 "User is locked out"` for `loginLockout` milliseconds.
pub(crate) async fn usable(
    state: &AppState,
    client: &Client,
    code: &str,
) -> Result<Invite, Refusal> {
    let limiter = state.login_limiter();
    let attempt = Attempt::new(client.address, LIMIT_KEY);
    if let Some(retry_after) = limiter.locked_for(&attempt, Instant::now()) {
        return Err(ApiError::LockedOut { retry_after }.into());
    }
    let found = if is_code(code) {
        state.database().invites().find(code).await?
    } else {
        None
    };
    if let Some(invite) = found.filter(|invite| invite.is_valid) {
        return Ok(invite);
    }
    tracing::warn!(client = %client.address, "an invite code was refused");
    Err(wrong_code(state, &attempt))
}

/// Counts a wrong code, and gives the answer for it.
fn wrong_code(state: &AppState, attempt: &Attempt) -> Refusal {
    let settings = state.settings();
    let policy = LimitPolicy {
        failures: settings.login_attempts,
        lockout: Duration::from_millis(settings.login_lockout),
    };
    match state
        .login_limiter()
        .record_failure(attempt, policy, Instant::now())
    {
        Some(retry_after) => ApiError::LockedOut { retry_after }.into(),
        None => Refusal::not_authorized("Code is invalid"),
    }
}

/// `GET /plugins/invites/{code}`: says whether a code can be used. Everybody can ask.
pub async fn verify(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    Path(code): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    check_enabled(&state)?;
    usable(&state, &client, &code).await?;
    Ok(Envelope::success(NoData).with_message("Code has been verified"))
}

/// The labels of a setting as plex.tv takes them.
fn label_filter(labels: &str) -> String {
    if labels.is_empty() {
        String::new()
    } else {
        format!("label={labels}")
    }
}

/// `POST /plugins/invites/{code}`: uses a code. The field `usedby` of the body is the
/// name or the e-mail address of the Plex account that gets access. Everybody can use
/// a code.
///
/// A code can be used once. When Plex does not give the access, the code can be used
/// again. The PHP application leaves such a code used.
pub async fn use_code(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    Path(code): Path<String>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    check_enabled(&state)?;
    let settings = state.settings();
    usable(&state, &client, &code).await?;
    let used_by = data
        .text("usedby")
        .map(|value| php_trim(&value).to_owned())
        .filter(|value| !value.is_empty() && value.chars().count() <= TEXT_LIMIT);
    let for_plex = settings.invites_type_include == "plex";
    let plex = if for_plex {
        if used_by.is_none() {
            return Err(Refusal::conflict("Plex username or email not supplied"));
        }
        if settings.plex_token.is_empty() || settings.plex_id.is_empty() {
            tracing::warn!("an invite cannot be used: the Plex token or the Plex id is not set");
            return Err(Refusal::conflict("Plex Token/ID not set"));
        }
        let Some(plex) = state.plex() else {
            return Err(Refusal::conflict("Plex cannot be asked"));
        };
        Some(plex)
    } else {
        None
    };

    // The extras of a share go to the address of the invite, as in PHP.
    let invite_email = invite_email(&state, &code).await?;
    reserve(&state, &client, &code, used_by.as_deref()).await?;
    tracing::info!(client = %client.address, "an invite was used");
    let (Some(plex), Some(account)) = (plex, used_by) else {
        // The PHP application does nothing more for a server that is not Plex.
        return Ok(Envelope::success(NoData).with_message("User now has access to system"));
    };
    match share_plex_server(&plex, &settings, &account).await {
        Ok(Access::Given) => {
            after_share(&state, &settings, &invite_email, &account).await;
            Ok(Envelope::success(NoData).with_message("Plex User now has access to system"))
        }
        // The account has the access, so the invite did its work.
        Ok(Access::AlreadyHad) => {
            after_share(&state, &settings, &invite_email, &account).await;
            Err(Refusal::conflict("Plex User already has access"))
        }
        Err(message) => {
            state.database().invites().restore(&code).await?;
            Err(Refusal::conflict(message))
        }
    }
}

/// Marks a usable code as used by `used_by`, from the address of the client. When
/// another request used the code a moment before, the answer is the one for a wrong
/// code, and it is counted.
pub(crate) async fn reserve(
    state: &AppState,
    client: &Client,
    code: &str,
    used_by: Option<&str>,
) -> Result<(), Refusal> {
    let now = now_stamp(SystemTime::now()).unix_seconds();
    let from = client.address.to_string();
    if state
        .database()
        .invites()
        .mark_used(code, used_by, &from, now)
        .await?
    {
        Ok(())
    } else {
        let attempt = Attempt::new(client.address, LIMIT_KEY);
        Err(wrong_code(state, &attempt))
    }
}

/// What a share of the Plex server did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    /// The account got the access.
    Given,
    /// The account had the access already.
    AlreadyHad,
}

/// The e-mail address of the invite with `code`, or the empty text.
pub(crate) async fn invite_email(state: &AppState, code: &str) -> Result<String, DbError> {
    Ok(state
        .database()
        .invites()
        .find(code)
        .await?
        .map_or_else(String::new, |invite| invite.email))
}

/// The longest wait for each service that a share sets up.
const EXTRA_TIME: Duration = Duration::from_secs(30);

/// What PHP's `_invitesPluginAction` does after it shares the Plex server: the accounts
/// in other services, each when its switch is on. A failure is logged; the invite stays
/// used, as in PHP.
pub(crate) async fn after_share(
    state: &AppState,
    settings: &Settings,
    email: &str,
    display_name: &str,
) {
    if settings.invites_add_plex_home {
        add_to_plex_home(state, settings, email).await;
    }
    let Some(http) = state.outbound() else {
        return;
    };
    if settings.invites_komga_enabled {
        make_komga_account(http, settings, email).await;
    }
    if settings.invites_nextcloud_enabled {
        make_nextcloud_user(state, http, settings, email, display_name).await;
    }
}

/// PHP's `_addUserPlexHome`: invites the address of the invite to the Plex Home of the
/// owner of the Plex token of the settings.
async fn add_to_plex_home(state: &AppState, settings: &Settings, email: &str) {
    if email.is_empty() || settings.plex_token.is_empty() {
        tracing::warn!(
            "no one is added to the Plex Home: the address or the Plex token is missing"
        );
        return;
    }
    let Some(plex) = state.plex() else {
        return;
    };
    match plex.add_to_home(&settings.plex_token, email).await {
        Ok(()) => tracing::info!("an invited account was added to the Plex Home"),
        Err(error) => tracing::warn!(%error, "an invited account was not added to the Plex Home"),
    }
}

/// PHP's `_createNextcloudAccount`: a user in Nextcloud for the address of the invite,
/// with the name that the person typed, a random password, the groups and the quota of
/// the settings. With the setting for the Plex app of Nextcloud, the id of the user is
/// `PlexTv-` and the id of the invite of Plex for that address.
async fn make_nextcloud_user(
    state: &AppState,
    http: &organizr_integrations::http::Outbound,
    settings: &Settings,
    email: &str,
    display_name: &str,
) {
    use organizr_integrations::nextcloud::{Admin, NewUser, create_user};

    if settings.invites_nextcloud_url.is_empty()
        || settings.invites_nextcloud_admin_user.is_empty()
        || settings.invites_nextcloud_admin_password.is_empty()
    {
        tracing::info!("no Nextcloud user is made: its address or its Admin account is not set");
        return;
    }
    let mut user_id = email.to_owned();
    if settings.invites_nextcloud_plex_sso
        && let Some(plex) = state.plex()
    {
        match plex.requested_invite_id(&settings.plex_token, email).await {
            Ok(Some(id)) => user_id = format!("PlexTv-{id}"),
            Ok(None) => tracing::warn!("no invite of Plex has the address of the invite"),
            Err(error) => tracing::warn!(%error, "the invites of Plex cannot be read"),
        }
    }
    let password = uuid::Uuid::new_v4().simple().to_string();
    let groups = list_of(&settings.invites_nextcloud_groups_member);
    let admin = Admin {
        url: settings.invites_nextcloud_url.trim_end_matches('/'),
        user: &settings.invites_nextcloud_admin_user,
        password: &settings.invites_nextcloud_admin_password,
    };
    let user = NewUser {
        user_id: &user_id,
        password: &password,
        email,
        display_name,
        groups: &groups,
        quota: &settings.invites_nextcloud_quota,
    };
    match create_user(http, admin, user, EXTRA_TIME).await {
        Ok(()) => tracing::info!("a Nextcloud user was made for an invite"),
        Err(error) => tracing::warn!(%error, "no Nextcloud user was made for an invite"),
    }
}

/// The parts of a list of the settings. The form saves a list with commas; PHP splits
/// it at semicolons, so both separate (D-118).
fn list_of(text: &str) -> Vec<String> {
    text.split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// PHP's `_createKomgaAccount`: an account in Komga for the address of the invite, when
/// every setting of it holds a value, as `_checkKomgaVar` asks.
async fn make_komga_account(
    http: &organizr_integrations::http::Outbound,
    settings: &Settings,
    email: &str,
) {
    let roles = list_of(&settings.invites_komga_roles);
    let library_ids = list_of(&settings.invites_komga_library_ids);
    if settings.invites_komga_uri.is_empty()
        || settings.invites_komga_api_key.is_empty()
        || settings.invites_komga_default_user_password.is_empty()
        || roles.is_empty()
        || library_ids.is_empty()
    {
        tracing::info!(
            "no Komga account is made: a setting of Komga in the invites plugin is empty"
        );
        return;
    }
    if email.is_empty() {
        tracing::info!("no Komga account is made: the invite has no e-mail address");
        return;
    }
    let account = organizr_integrations::komga::NewAccount {
        email,
        password: &settings.invites_komga_default_user_password,
        roles: &roles,
        library_ids: &library_ids,
    };
    let url = settings.invites_komga_uri.trim_end_matches('/');
    match organizr_integrations::komga::create_account(
        http,
        url,
        &settings.invites_komga_api_key,
        account,
        EXTRA_TIME,
    )
    .await
    {
        Ok(()) => {
            tracing::info!(roles = %roles.join(","), "a Komga account was made for an invite");
        }
        Err(error) => {
            tracing::warn!(%error, "no Komga account was made for an invite");
        }
    }
}

/// Shares the Plex server of the settings with an account, with the libraries and the
/// labels of the invites plugin. A failure is the message for the caller.
pub(crate) async fn share_plex_server(
    plex: &PlexTv,
    settings: &Settings,
    account: &str,
) -> Result<Access, &'static str> {
    let libraries: Vec<&str> = settings
        .invites_plex_libraries
        .split(',')
        .filter(|library| !library.is_empty())
        .collect();
    let share = Share {
        account,
        libraries: &libraries,
        tv_filter: &label_filter(&settings.invites_plex_tv_labels),
        movies_filter: &label_filter(&settings.invites_plex_movies_labels),
        music_filter: &label_filter(&settings.invites_plex_music_labels),
    };
    let message = match plex
        .share(&settings.plex_token, &settings.plex_id, &share)
        .await
    {
        Ok(()) => return Ok(Access::Given),
        Err(PlexError::Status(400)) => return Ok(Access::AlreadyHad),
        Err(PlexError::Status(401)) => "Incorrect Token",
        Err(PlexError::Status(404)) => "Libraries not setup correct",
        Err(PlexError::Status(_)) => "An Error Occurred",
        Err(_) => "Plex cannot be reached",
    };
    tracing::warn!(
        reason = message,
        "Plex did not give the access of an invite"
    );
    Err(message)
}

/// `DELETE /plugins/invites/{code}`: deletes an invite. An Admin or a Co-Admin deletes
/// each invite. Another member deletes the own invites, when the settings allow that.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(code): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    check_member(&state, &caller)?;
    let invites = state.database().invites();
    let manager = is_manager(&caller);
    if !manager && !state.settings().invites_allow_delete_include {
        return Err(Refusal::conflict(
            "You are not permitted to delete invites.",
        ));
    }
    let found = invites
        .find(&code)
        .await?
        .filter(|invite| manager || invite.invited_by == name_of(&caller));
    if found.is_none() || !invites.delete(&code).await? {
        return Err(Refusal::not_found("Code not found"));
    }
    Ok(Envelope::success(NoData).with_message("Code has been deleted"))
}

/// The longest wait for the lists of Komga and Nextcloud.
const LIST_TIME: Duration = Duration::from_secs(10);

/// The groups of the Nextcloud of the plugin as choices, or nothing when it is not set up
/// or does not answer, as PHP's `_checkNextcloudVar` asks.
pub(crate) async fn nextcloud_groups(state: &AppState, settings: &Settings) -> Option<Vec<Choice>> {
    use organizr_integrations::nextcloud::{Admin, groups};

    if !settings.invites_nextcloud_enabled
        || settings.invites_nextcloud_url.is_empty()
        || settings.invites_nextcloud_admin_user.is_empty()
        || settings.invites_nextcloud_admin_password.is_empty()
    {
        return None;
    }
    let http = state.outbound()?;
    let admin = Admin {
        url: settings.invites_nextcloud_url.trim_end_matches('/'),
        user: &settings.invites_nextcloud_admin_user,
        password: &settings.invites_nextcloud_admin_password,
    };
    match groups(http, admin, LIST_TIME).await {
        Ok(found) => Some(
            found
                .into_iter()
                .map(|group| Choice {
                    label: group.clone(),
                    value: group,
                })
                .collect(),
        ),
        Err(error) => {
            tracing::warn!(%error, "the groups of Nextcloud cannot be read");
            None
        }
    }
}

/// The libraries of the Komga of the plugin as choices, or nothing when it is not set up
/// or does not answer. PHP asks the address of single sign-on, `komgaURL`; the address of
/// the plugin, `INVITES-komga-uri`, comes first here, as its account goes there too
/// (D-118).
pub(crate) async fn komga_libraries(state: &AppState, settings: &Settings) -> Option<Vec<Choice>> {
    let url = [&settings.invites_komga_uri, &settings.komga_url]
        .into_iter()
        .find(|url| !url.is_empty())?;
    if settings.invites_komga_api_key.is_empty() {
        return None;
    }
    let http = state.outbound()?;
    match organizr_integrations::komga::libraries(
        http,
        url.trim_end_matches('/'),
        &settings.invites_komga_api_key,
        LIST_TIME,
    )
    .await
    {
        Ok(libraries) => Some(
            libraries
                .into_iter()
                .map(|library| Choice {
                    label: library.name,
                    value: library.id,
                })
                .collect(),
        ),
        Err(error) => {
            tracing::warn!(%error, "the libraries of Komga cannot be read");
            None
        }
    }
}

/// `GET /plugins/invites/nextcloud/groups`: the groups of the Nextcloud of the plugin, for
/// Admins and Co-Admins, as a list of `{label, value}`. The list is empty when Nextcloud is
/// not set up or does not answer. Not a route of PHP: its form holds the list.
pub async fn nextcloud_group_list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<Choice>>, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    Ok(Envelope::success(
        nextcloud_groups(&state, &settings)
            .await
            .unwrap_or_default(),
    ))
}

/// `GET /plugins/invites/komga/libraries`: the libraries of the Komga of the plugin, for
/// Admins and Co-Admins, as a list of `{label, value}`. The list is empty when Komga is not
/// set up or does not answer. Not a route of PHP: its form holds the list.
pub async fn komga_library_list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<Choice>>, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    Ok(Envelope::success(
        komga_libraries(&state, &settings).await.unwrap_or_default(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_holds_letters_digits_and_two_signs() {
        assert!(is_code("AbC-12_3"));
        assert!(is_code(&"a".repeat(CODE_LIMIT)));
        for text in ["", "a b", "a/b", "ä", "a?b", &"a".repeat(CODE_LIMIT + 1)] {
            assert!(!is_code(text), "{text}");
        }
    }

    #[test]
    fn times_and_labels_are_written_as_php_writes_them() {
        assert_eq!(date_and_time(1_795_000_000), "2026-11-18 11:06:40");
        assert_eq!(label_filter(""), "");
        assert_eq!(label_filter("kids,family"), "label=kids,family");
    }
}
