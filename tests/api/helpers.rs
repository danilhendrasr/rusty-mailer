use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Algorithm, Argon2, Params, PasswordHasher, Version,
};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DBSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

pub struct ConfirmationLink {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestUser {
    id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            r#"INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)"#,
            self.id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to insert test user.");
    }
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
    pub http_client: reqwest::Client,
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        self.http_client
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletter(&self, body: &serde_json::Value) -> reqwest::Response {
        self.http_client
            .post(format!("{}/newsletters", self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(body)
            .send()
            .await
            .expect("Failed to send reqwest to /newsletters")
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.http_client
            .post(format!("{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to send reqwest to /login")
    }

    pub fn get_confirmation_link_from_email_body(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLink {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_links = |s: &str| {
            let links = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect::<Vec<_>>();

            assert_eq!(links.len(), 1);
            let raw_confirmation_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_confirmation_link).unwrap();

            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_links(&body["html_body"].as_str().unwrap());
        let plain_text = get_links(&body["text_body"].as_str().unwrap());

        ConfirmationLink { html, plain_text }
    }

    pub async fn get_login_html(&self) -> String {
        self.http_client
            .get(format!("{}/login", self.address))
            .send()
            .await
            .expect("Failed getting the login form HTML")
            .text()
            .await
            .unwrap()
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to get configurations");
        c.database.db_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_db(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");

    let application_port = application.port();
    let address = format!("http://127.0.0.1:{}", application.port());
    tokio::spawn(application.run_until_stopped());

    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app_instance = TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: application_port,
        test_user: TestUser::generate(),
        http_client,
    };

    test_app_instance
        .test_user
        .store(&test_app_instance.db_pool)
        .await;

    test_app_instance
}

async fn configure_db(settings: &DBSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&settings.without_db())
        .await
        .expect("Failed connecting to the database");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, settings.db_name).as_str())
        .await
        .expect(format!("Failed to create {} database", settings.db_name).as_str());

    let connection_pool = PgPool::connect_with(settings.with_db())
        .await
        .expect(format!("Failed to connect to database {}", settings.db_name).as_str());

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
