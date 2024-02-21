use secrecy::ExposeSecret;
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    // pub application_port: u16,
    pub application: ApplicationSettings,
}

// reminder of general DB connection string values / `shape`:
// postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    // helper for formatted Postgres connection string
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        ))
    }

    // helper for formatted Postgres connection string - NO NAME (for randomization / testing)
    pub fn connection_string_without_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        ))
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine current directory");
    let config_dir = base_path.join("configuration");

    // detect running env
    let env: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT variable");
    let env_filename = format!("{}.yaml", env.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(config_dir.join("base.yaml")))
        .add_source(config::File::from(config_dir.join(env_filename)))
        .build()?;

    settings.try_deserialize::<Settings>()
}

// pub fn get_configuration() -> Result<Settings, config::ConfigError> {
//     // init config reader
//     let settings = config::Config::builder()
//         // add config values from our configuration yaml file
//         .add_source(config::File::new(
//             "configuration.yaml",
//             config::FileFormat::Yaml,
//         ))
//         .build()?;
//     // try to deserialize into our `Settings` type
//     settings.try_deserialize::<Settings>()
// }

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment.Use either `local` or `production`",
                other
            )),
        }
    }
}
