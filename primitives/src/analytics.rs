use crate::DomainError;
use serde::{Deserialize, Serialize};

pub const ANALYTICS_QUERY_LIMIT: u32 = 200;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub time: f64,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub aggr: Vec<AnalyticsData>,
    pub limit: u32
}

#[cfg(feature = "postgres")]
pub mod postgres {
    use super::AnalyticsData;
    use tokio_postgres::Row;

    impl From<&Row> for AnalyticsData {
        fn from(row: &Row) -> Self {
            Self {
                time: row.get("time"),
                value: row.get("value"),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default = "default_event_type")]
    pub event_type: String,
    #[serde(default = "default_metric")]
    pub metric: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    pub segment_by_channel: Option<String>,
}

impl AnalyticsQuery {
    pub fn metric_to_column(&mut self) {
        self.metric = match self.metric.as_str() {
            "eventCounts"=> "count".to_string(),
            "eventPayouts" => "payout".to_string(),
            _ => "count".to_string(),
        };
    }
    pub fn is_valid(&self) -> Result<(), DomainError> {
        let valid_event_types = ["IMPRESSION", "CLICK"];
        let valid_metric = ["eventPayouts", "eventCounts"];
        let valid_timeframe = ["year", "month", "week", "day", "hour"];

        if !valid_event_types.contains(&self.event_type.as_str()) {
            Err(DomainError::InvalidArgument(format!(
                "invalid event_type, possible values are: {}",
                valid_event_types.join(" ,")
            )))
        } else if !valid_metric.contains(&self.metric.as_str()) {
            Err(DomainError::InvalidArgument(format!(
                "invalid metric, possible values are: {}",
                valid_metric.join(" ,")
            )))
        } else if !valid_timeframe.contains(&self.timeframe.as_str()) {
            Err(DomainError::InvalidArgument(format!(
                "invalid timeframe, possible values are: {}",
                valid_timeframe.join(" ,")
            )))
        } else if self.limit > ANALYTICS_QUERY_LIMIT {
            Err(DomainError::InvalidArgument(format!(
                "invalid limit {}, maximum value 200",
                self.limit
            )))
        } else {
            Ok(())
        }
    }
}

fn default_limit() -> u32 {
    100
}

fn default_event_type() -> String {
    "IMPRESSION".into()
}

fn default_metric() -> String {
    "eventCounts".into()
}

fn default_timeframe() -> String {
    "hour".into()
}
