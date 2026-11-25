//! The `reset-second-factor` command: removes the second factor of a user, for a person
//! who lost the authenticator app and cannot log in. It is the way back in when nobody
//! who could remove the factor in Manage Users can log in, for example the only Admin.

use anyhow::Context;
use organizr_db::Database;

use crate::serve::{StorageArgs, open_database};

/// The options of `organizr reset-second-factor`.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The name of the user, in any letter case.
    #[arg(long)]
    user: String,

    #[command(flatten)]
    storage: StorageArgs,
}

/// Removes the second factor and prints what was done.
pub async fn run(args: Args) -> anyhow::Result<()> {
    let database = open_database(&args.storage).await?;
    let outcome = reset(&database, &args.user).await;
    database
        .close()
        .await
        .context("the database did not close cleanly")?;
    println!("{}", outcome?);
    Ok(())
}

/// Removes the second factor of the user with the name, and says what was done.
async fn reset(database: &Database, name: &str) -> anyhow::Result<String> {
    let users = database.users();
    let user = users
        .find_by_username(name.trim())
        .await
        .context("cannot look the user up")?
        .with_context(|| format!("there is no user with the name {}", name.trim()))?;
    let had_one = users
        .login_of(user.id)
        .await
        .context("cannot read the login of the user")?
        .is_some_and(|login| login.two_factor.is_some());
    if !had_one {
        return Ok(format!(
            "The user {} has no second factor. Nothing was changed.",
            user.username
        ));
    }
    users
        .set_two_factor(user.id, None)
        .await
        .context("cannot remove the second factor")?;
    tracing::info!(user = %user.username, "the second factor of a user was removed on the command line");
    Ok(format!(
        "The second factor of {} is removed. The user logs in with the password alone \
         and can set up a new second factor in the account settings.",
        user.username
    ))
}

#[cfg(test)]
mod tests {
    use organizr_db::SealedTwoFactor;
    use organizr_testkit::engines::on_each_engine;
    use organizr_testkit::seed;

    use super::reset;

    #[tokio::test]
    async fn second_factor_of_the_named_user_is_removed() {
        on_each_engine(async |database| {
            seed::installed(database).await;
            let users = database.users();
            let admin = users
                .find_by_username("admin")
                .await
                .expect("the lookup works")
                .expect("the install has an Admin");
            let factor = SealedTwoFactor {
                kind: "google".to_owned(),
                sealed_secret: "sealed".to_owned(),
            };
            users
                .set_two_factor(admin.id, Some(factor))
                .await
                .expect("the factor is stored");

            let done = reset(database, " ADMIN ").await.expect("the reset works");
            assert!(
                done.starts_with("The second factor of admin is removed."),
                "{done}"
            );
            let login = users
                .login_of(admin.id)
                .await
                .expect("the login loads")
                .expect("the user is there");
            assert_eq!(login.two_factor, None);

            let again = reset(database, "admin").await.expect("the reset works");
            assert_eq!(
                again,
                "The user admin has no second factor. Nothing was changed."
            );
            let missing = reset(database, "nobody")
                .await
                .expect_err("there is no such user");
            assert_eq!(missing.to_string(), "there is no user with the name nobody");
        })
        .await;
    }
}
