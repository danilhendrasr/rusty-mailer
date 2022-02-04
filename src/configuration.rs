#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DBSettings,
    pub application_port: u16,
}

#[derive(serde::Deserialize, Debug)]
pub struct DBSettings {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub db_name: String,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("configuration"))?;
    settings.try_into()
}

impl DBSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.db_name
        )
    }

    pub fn connection_string_wo_db(&self) -> String {
        println!("{:?}", self);
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}
