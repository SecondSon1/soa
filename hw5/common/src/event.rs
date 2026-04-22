use apache_avro::Schema;
use apache_avro::types::Record;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    ViewStarted,
    ViewFinished,
    ViewPaused,
    ViewResumed,
    Liked,
    Searched,
}

impl EventType {
    fn avro_index(&self) -> u32 {
        match self {
            Self::ViewStarted => 0,
            Self::ViewFinished => 1,
            Self::ViewPaused => 2,
            Self::ViewResumed => 3,
            Self::Liked => 4,
            Self::Searched => 5,
        }
    }

    fn as_avro_str(&self) -> &'static str {
        match self {
            Self::ViewStarted => "VIEW_STARTED",
            Self::ViewFinished => "VIEW_FINISHED",
            Self::ViewPaused => "VIEW_PAUSED",
            Self::ViewResumed => "VIEW_RESUMED",
            Self::Liked => "LIKED",
            Self::Searched => "SEARCHED",
        }
    }

    fn to_avro(&self) -> apache_avro::types::Value {
        apache_avro::types::Value::Enum(self.avro_index(), self.as_avro_str().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceType {
    Mobile,
    Desktop,
    Tv,
    Tablet,
}

impl DeviceType {
    fn avro_index(&self) -> u32 {
        match self {
            Self::Mobile => 0,
            Self::Desktop => 1,
            Self::Tv => 2,
            Self::Tablet => 3,
        }
    }

    fn as_avro_str(&self) -> &'static str {
        match self {
            Self::Mobile => "MOBILE",
            Self::Desktop => "DESKTOP",
            Self::Tv => "TV",
            Self::Tablet => "TABLET",
        }
    }

    fn to_avro(&self) -> apache_avro::types::Value {
        apache_avro::types::Value::Enum(self.avro_index(), self.as_avro_str().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieEvent {
    pub event_id: Uuid,
    pub user_id: String,
    pub movie_id: String,
    pub event_type: EventType,
    pub timestamp: i64, // epoch milliseconds
    pub device_type: DeviceType,
    pub session_id: String,
    pub progress_seconds: i32,
}

impl MovieEvent {
    pub fn to_avro_record<'a>(&self, schema: &'a Schema) -> anyhow::Result<Record<'a>> {
        let mut record = Record::new(schema)
            .ok_or_else(|| anyhow::anyhow!("failed to create Avro record from schema"))?;
        record.put("event_id", apache_avro::types::Value::Uuid(self.event_id));
        record.put("user_id", self.user_id.as_str());
        record.put("movie_id", self.movie_id.as_str());
        record.put("event_type", self.event_type.to_avro());
        record.put(
            "timestamp",
            apache_avro::types::Value::TimestampMillis(self.timestamp),
        );
        record.put("device_type", self.device_type.to_avro());
        record.put("session_id", self.session_id.as_str());
        record.put("progress_seconds", self.progress_seconds);
        Ok(record)
    }
}
