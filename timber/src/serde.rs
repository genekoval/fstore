#![cfg(feature = "serde")]

use crate::Sink;

use serde::{
    de::{Deserialize, Error, Visitor},
    ser::{Serialize, Serializer},
};
use std::{
    fmt::{self, Formatter},
    path::PathBuf,
};

impl Serialize for Sink {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Sink::Stderr => serializer.serialize_str("stderr"),
            Sink::Stdout => serializer.serialize_str("stdout"),
            Sink::Syslog(_) => serializer.serialize_str("syslog"),
            Sink::File(path) => {
                serializer.serialize_str(path.to_str().unwrap())
            }
        }
    }
}

impl<'de> Deserialize<'de> for Sink {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SinkVisitor;

        impl<'de> Visitor<'de> for SinkVisitor {
            type Value = Sink;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("log sink")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(match value {
                    "stderr" => Sink::Stderr,
                    "stdout" => Sink::Stdout,
                    "syslog" => Sink::Syslog(Default::default()),
                    _ => Sink::File(PathBuf::from(value)),
                })
            }
        }

        deserializer.deserialize_str(SinkVisitor)
    }
}
