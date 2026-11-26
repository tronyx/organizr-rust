//! The notice of an import: the breaking changes that `organizr import` found for the
//! install, which the Admin sees after the login until an Admin hides them (D-158).

use axum::Extension;
use axum::extract::State;
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The lines of the notice, or `None` when there is no notice.
pub fn lines(settings: &Settings) -> Option<Vec<String>> {
    let lines: Vec<String> = serde_json::from_str(&settings.import_notice).ok()?;
    (!lines.is_empty()).then_some(lines)
}

/// `DELETE /import-notice`: an Admin hides the notice for every Admin.
pub async fn hide(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(GroupLevel::ADMIN)?;
    let change = vec![("importNotice".to_owned(), Value::String(String::new()))];
    state
        .change_settings(change)
        .await
        .map_err(ApiError::from)?;
    tracing::info!("the notice of the import was hidden");
    Ok(Envelope::success(NoData).with_message("The notice of the import is hidden"))
}

#[cfg(test)]
mod tests {
    use organizr_settings::Settings;

    use super::lines;

    #[test]
    fn notice_is_a_list_of_texts_that_is_not_empty() {
        let mut settings = Settings::defaults();
        assert_eq!(lines(&settings), None);
        for empty in ["[]", "not a list", "{\"a\":1}"] {
            settings.import_notice = empty.to_owned();
            assert_eq!(lines(&settings), None, "{empty}");
        }
        settings.import_notice = r#"["One (D-154).","Two."]"#.to_owned();
        assert_eq!(
            lines(&settings),
            Some(vec!["One (D-154).".to_owned(), "Two.".to_owned()])
        );
    }
}
