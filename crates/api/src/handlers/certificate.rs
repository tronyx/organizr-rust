//! `POST /certificate/custom`: the upload of the certificate of a certificate authority,
//! as PHP's `uploadCert()`.
//!
//! The requests to a service whose setting "use the custom certificate" is on then also
//! trust the certificates that this authority signed. PHP stores any file whose name
//! ends with `.pem` and lets Co-Admins upload it. Here only the Admin uploads, because
//! the file changes what the server trusts, and the file must be the certificate of an
//! authority (see `organizr_integrations::custom_certificate`). `docs/deviations.md`
//! lists the differences as D-98.

use axum::Extension;
use axum::extract::State;
use axum::extract::multipart::{Multipart, MultipartRejection};
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use organizr_integrations::custom_certificate::CustomCertificate;
use organizr_integrations::http::Outbound;

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::state::AppState;

pub use organizr_integrations::custom_certificate::LARGEST;

/// The last part of a path, as PHP's `pathinfo(..., PATHINFO_BASENAME)`.
fn base_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or_default()
}

/// The refusal of PHP for a file that is not taken: `403`.
fn not_approved(name: &str) -> Refusal {
    Refusal::Request(
        StatusCode::FORBIDDEN,
        format!("{name} is not approved to be uploaded"),
    )
}

fn too_large() -> Refusal {
    Refusal::Request(
        StatusCode::PAYLOAD_TOO_LARGE,
        format!("The certificate file has {LARGEST} bytes at most"),
    )
}

fn upload_error() -> Refusal {
    Refusal::Request(
        StatusCode::INTERNAL_SERVER_ERROR,
        "File upload error".to_owned(),
    )
}

/// The name and the content of the file in the field `file`.
async fn read_file(mut parts: Multipart) -> Result<Option<(String, Vec<u8>)>, Refusal> {
    loop {
        let part = match parts.next_field().await {
            Ok(Some(part)) => part,
            Err(error) if error.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                return Err(too_large());
            }
            Ok(None) | Err(_) => return Ok(None),
        };
        if part.name() != Some("file") {
            continue;
        }
        let Some(name) = part.file_name().map(str::to_owned) else {
            continue;
        };
        return match part.bytes().await {
            Ok(content) if content.len() > LARGEST => Err(too_large()),
            Ok(content) => Ok(Some((name, content.to_vec()))),
            Err(error) if error.status() == StatusCode::PAYLOAD_TOO_LARGE => Err(too_large()),
            Err(_) => Err(upload_error()),
        };
    }
}

/// `POST /certificate/custom`: stores the certificate of the field `file` and trusts
/// its authority from now on.
pub async fn upload(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    parts: Result<Multipart, MultipartRejection>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(GroupLevel::ADMIN)?;
    // PHP names the file that it did not get with an empty name.
    let Some((name, content)) = (match parts {
        Ok(parts) => read_file(parts).await?,
        Err(_) => None,
    }) else {
        return Err(not_approved(""));
    };
    let name = base_name(&name).to_owned();
    let is_pem = name
        .rsplit_once('.')
        .is_some_and(|(_, ending)| ending.eq_ignore_ascii_case("pem"));
    if !is_pem {
        return Err(not_approved(&name));
    }
    let certificate = CustomCertificate::parse(&content)
        .map_err(|error| Refusal::unprocessable(&error.to_string()))?;
    let (Some(file), Some(outbound)) = (state.custom_certificate_file(), state.outbound()) else {
        return Err(upload_error());
    };
    // The client is made first, so a certificate that it cannot use is not stored.
    if Outbound::check_custom_certificate(&certificate).is_err() {
        return Err(Refusal::unprocessable(
            "The certificate cannot be used for the requests",
        ));
    }
    let stored = async {
        if let Some(folder) = file.parent() {
            tokio::fs::create_dir_all(folder).await?;
        }
        let part = file.with_extension("pem.part");
        tokio::fs::write(&part, certificate.pem()).await?;
        tokio::fs::rename(&part, &file).await
    };
    if let Err(error) = stored.await {
        tracing::error!(%error, "the uploaded certificate could not be stored");
        return Err(upload_error());
    }
    if let Err(error) = outbound.set_custom_certificate(Some(&certificate)) {
        tracing::error!(error = %error.without_url(), "the uploaded certificate could not be used");
        return Err(upload_error());
    }
    tracing::info!(
        certificates = certificate.count(),
        "the certificate of an authority was uploaded"
    );
    Ok(Envelope::success(NoData).with_message(format!("{name} has been uploaded")))
}
