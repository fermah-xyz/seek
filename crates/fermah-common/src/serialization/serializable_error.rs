use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Serialize};

pub enum SerializableErrorWrapper<E> {
    Actual(E),
    Stringified(String),
}

impl<E> SerializableErrorWrapper<E> {
    pub fn unwrap(self) -> E {
        match self {
            Self::Actual(e) => e,
            Self::Stringified(_) => panic!("Can't unwrap stringified error!"),
        }
    }
}

impl<E: Display + Debug> std::error::Error for SerializableErrorWrapper<E> {}

impl<E> From<E> for SerializableErrorWrapper<E> {
    fn from(error: E) -> Self {
        Self::Actual(error)
    }
}

impl<E: Display> Display for SerializableErrorWrapper<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actual(e) => write!(f, "{}", e),
            Self::Stringified(e_str) => write!(f, "{}", e_str),
        }
    }
}

impl<E: Debug> Debug for SerializableErrorWrapper<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actual(e) => write!(f, "SerializableErrorWrapper::Actual({:?})", e),
            Self::Stringified(e_str) => {
                write!(f, "SerializableErrorWrapper::Stringified({})", e_str)
            }
        }
    }
}

impl<E: Debug> Serialize for SerializableErrorWrapper<E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Actual(e) => serializer.serialize_str(&format!("{e:?}")),
            Self::Stringified(e_str) => serializer.serialize_str(e_str),
        }
    }
}

impl<'de, E> Deserialize<'de> for SerializableErrorWrapper<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(SerializableErrorWrapper::Stringified(s))
    }
}

pub trait WrapError<T, E> {
    fn wrap(self) -> Result<T, SerializableErrorWrapper<E>>
    where
        Self: Sized,
        E: Sized + std::error::Error + Debug + Display;
}

impl<T, E> WrapError<T, E> for Result<T, E>
where
    E: std::error::Error + Debug + Display,
{
    fn wrap(self) -> Result<T, SerializableErrorWrapper<E>> {
        self.map_err(SerializableErrorWrapper::from)
    }
}

#[cfg(test)]
mod tests {

    use serde::{Deserialize, Serialize};
    use strum::Display;
    use thiserror::Error;

    use super::*;

    #[allow(dead_code)]
    #[derive(Debug, Display)]
    enum OtherError {
        CaseA(String),
    }

    impl std::error::Error for OtherError {}

    #[derive(Error, Debug, Serialize, Deserialize)]
    enum SomeError {
        #[error("IO error: {0}")]
        Io(#[from] SerializableErrorWrapper<std::io::Error>),

        #[error("Reqwest error: {0}")]
        Reqwest(#[from] SerializableErrorWrapper<reqwest::Error>),

        #[error("Other error: {0}")]
        Other(#[from] SerializableErrorWrapper<OtherError>),
    }

    #[test]
    fn serde_wrap_test() {
        fn raise_error() -> Result<(), SomeError> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Uh Oh")).wrap()?
        }
        let e = raise_error().unwrap_err();
        println!("Error occurred: {:?}", e);
        let serialized = serde_json::to_string(&e).expect("Failed to serialize error");
        println!("Serialized error: {}", serialized);
        let deserialized: SomeError =
            serde_json::from_str(&serialized).expect("Failed to deserialize error");
        println!("Deserialized error: {:?}", deserialized);

        fn raise_other_error() -> Result<(), SomeError> {
            let a: bool = rand::random();
            if a {
                Err(OtherError::CaseA("Case a".to_string())).wrap()?
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Uh Oh")).wrap()?
            }
        }
        let e = raise_other_error().unwrap_err();
        println!("Error occurred: {:?}", e);
        let serialized = serde_json::to_string(&e).expect("Failed to serialize error");
        println!("Serialized error: {}", serialized);
        let deserialized: SomeError =
            serde_json::from_str(&serialized).expect("Failed to deserialize error");
        println!("Deserialized error: {:?}", deserialized);
    }
}
