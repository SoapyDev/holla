#[derive(Debug)]
pub enum HollaError {
    IO(std::io::Error),
    NotInstalled(&'static str),
    UserNotFound,
}

impl From<std::io::Error> for HollaError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}
