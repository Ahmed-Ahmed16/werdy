use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use werdy_core::WerdyDay;

use crate::{Accepted, DeedKey, Stamp};

/// A stream row is readable or held until a build can decode its change.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BatchRow {
    Readable(Accepted),
    Held(HeldRow),
}

impl BatchRow {
    pub fn accepted(&self) -> Option<&Accepted> {
        match self {
            Self::Readable(row) => Some(row),
            Self::Held(_) => None,
        }
    }

    pub fn stamp(&self) -> Stamp {
        match self {
            Self::Readable(row) => row.stamp,
            Self::Held(row) => row.stamp,
        }
    }
}

impl From<Accepted> for BatchRow {
    fn from(row: Accepted) -> Self {
        Self::Readable(row)
    }
}

impl<'de> Deserialize<'de> for BatchRow {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        match serde_json::from_str(raw.get()) {
            Ok(row) => Ok(Self::Readable(row)),
            Err(_) => HeldRow::try_from(raw)
                .map(Self::Held)
                .map_err(serde::de::Error::custom),
        }
    }
}

/// Original JSON, including numbers wider than a JSON value can hold exactly.
#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "Box<RawValue>")]
pub struct HeldRow {
    raw: Box<RawValue>,
    pub(crate) stamp: Stamp,
    pub(crate) install: String,
    pub(crate) operation: u64,
    pub(crate) dependency: Option<DeedDependency>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) enum DeedDependency {
    Deed { id: DeedKey },
    Rule { id: DeedKey, from_day: WerdyDay },
}

impl HeldRow {
    pub(crate) fn decode(&self) -> Option<Accepted> {
        serde_json::from_str(self.raw.get()).ok()
    }
}

impl TryFrom<Box<RawValue>> for HeldRow {
    type Error = serde_json::Error;

    fn try_from(raw: Box<RawValue>) -> Result<Self, Self::Error> {
        #[derive(Deserialize)]
        struct Envelope<'a> {
            #[serde(borrow)]
            pending: Operation<'a>,
            stamp: Stamp,
        }

        #[derive(Deserialize)]
        struct Operation<'a> {
            install: String,
            operation: u64,
            #[serde(borrow)]
            change: &'a RawValue,
        }

        let Envelope { pending, stamp } = serde_json::from_str(raw.get())?;
        let Operation {
            install,
            operation,
            change,
        } = pending;
        let dependency = serde_json::from_str(change.get()).ok();
        Ok(Self {
            raw,
            stamp,
            install,
            operation,
            dependency,
        })
    }
}

impl PartialEq for HeldRow {
    fn eq(&self, other: &Self) -> bool {
        self.raw.get() == other.raw.get()
    }
}

impl Serialize for HeldRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(serializer)
    }
}
