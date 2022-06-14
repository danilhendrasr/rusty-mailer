use anyhow::Context;
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use rand::rngs::OsRng;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct UserCredentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validating credentials.", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: UserCredentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_user_credentials(&credentials.username, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    // We don't exit early upon finding that the username does not exist,
    // instead, we do the same amount of work whether the username is found or not.
    // This is done to minimize the chance of getting timing attacked.
    user_id
        .ok_or_else(|| anyhow::anyhow!("Username not found."))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored user credentials.", skip(username, pool))]
pub async fn get_stored_user_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT id, password_hash FROM users WHERE username = $1"#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed fetching stored user credentials from the database.")?
    .map(|row| (row.id, Secret::new(row.password_hash)));

    Ok(row)
}

#[tracing::instrument(name = "Change password", skip(db_pool))]
pub async fn change_password(
    user_id: Uuid,
    new_password: Secret<String>,
    db_pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(new_password))
        .await?
        .context("Failed computing hash for the new password.")?;

    sqlx::query!(
        r#"UPDATE users SET password_hash = $1 WHERE id = $2"#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(db_pool)
    .await
    .context("Failed changing user's password.")?;

    Ok(())
}

#[tracing::instrument(
    name = "Verify password hash.",
    skip(expected_password_hash, password_candidate)
)]
pub fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed parsing password hash from PHC string.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(Secret::new(password_hash))
}
