use anyhow::Result;
use chrono::NaiveDate;
use clickhouse::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct MetricResult {
    pub date: NaiveDate,
    pub metric_name: String,
    pub dimension: String,
    pub value: f64,
}

#[derive(Row, Deserialize)]
struct SingleF64 {
    value: f64,
}

#[derive(Row, Deserialize)]
struct SingleU64 {
    value: u64,
}

#[derive(Row, Deserialize)]
struct DimensionU64 {
    dimension: String,
    value: u64,
}

#[derive(Row, Deserialize)]
struct DateRow {
    #[serde(with = "clickhouse::serde::chrono::date")]
    d: NaiveDate,
}

pub async fn get_all_dates(ch: &clickhouse::Client) -> Result<Vec<NaiveDate>> {
    let dates = ch
        .query("SELECT DISTINCT toDate(timestamp) as d FROM movie_events ORDER BY d")
        .fetch_all::<DateRow>()
        .await?;
    Ok(dates.into_iter().map(|r| r.d).collect())
}

pub async fn compute_for_date(
    ch: &clickhouse::Client,
    date: NaiveDate,
) -> Result<Vec<MetricResult>> {
    let mut results = Vec::new();
    let date_str = date.format("%Y-%m-%d").to_string();

    // DAU
    let dau = ch
        .query("SELECT uniq(user_id) as value FROM movie_events WHERE toDate(timestamp) = ?")
        .bind(date_str.as_str())
        .fetch_one::<SingleU64>()
        .await?;
    results.push(MetricResult {
        date,
        metric_name: "dau".into(),
        dimension: String::new(),
        value: dau.value as f64,
    });

    // Average watch time
    let avg_watch = ch
        .query(
            "SELECT ifNull(avg(progress_seconds), 0) as value FROM movie_events \
             WHERE event_type = 'VIEW_FINISHED' AND toDate(timestamp) = ?",
        )
        .bind(date_str.as_str())
        .fetch_one::<SingleF64>()
        .await?;
    results.push(MetricResult {
        date,
        metric_name: "avg_watch_time".into(),
        dimension: String::new(),
        value: avg_watch.value,
    });

    // Top movies
    let top_movies = ch
        .query(
            "SELECT movie_id as dimension, count() as value FROM movie_events \
             WHERE event_type = 'VIEW_STARTED' AND toDate(timestamp) = ? \
             GROUP BY movie_id ORDER BY value DESC",
        )
        .bind(date_str.as_str())
        .fetch_all::<DimensionU64>()
        .await?;
    for m in top_movies {
        results.push(MetricResult {
            date,
            metric_name: "top_movies".into(),
            dimension: m.dimension,
            value: m.value as f64,
        });
    }

    // View conversion
    let conversion = ch
        .query(
            "SELECT ifNull(countIf(event_type = 'VIEW_FINISHED') / \
             nullIf(countIf(event_type = 'VIEW_STARTED'), 0), 0) as value \
             FROM movie_events \
             WHERE event_type IN ('VIEW_STARTED', 'VIEW_FINISHED') \
             AND toDate(timestamp) = ?",
        )
        .bind(date_str.as_str())
        .fetch_one::<SingleF64>()
        .await?;
    results.push(MetricResult {
        date,
        metric_name: "view_conversion".into(),
        dimension: String::new(),
        value: conversion.value,
    });

    // Cohort retention: treat `date` as the cohort date, compute Day 0–7
    let today = chrono::Utc::now().date_naive();
    let cohort_str = date.format("%Y-%m-%d").to_string();
    for offset in 0i64..=7 {
        let target_date = date + chrono::Duration::days(offset);
        if target_date > today {
            break;
        }
        let retention = if offset == 0 {
            1.0
        } else {
            let target_str = target_date.format("%Y-%m-%d").to_string();
            let result = ch
                .query(
                    "SELECT if(cohort = 0, 0.0, returned / cohort) as value FROM ( \
                        SELECT \
                            count() as cohort, \
                            countIf(has_target > 0) as returned \
                        FROM ( \
                            SELECT user_id, \
                                min(toDate(timestamp)) as first_date, \
                                countIf(toDate(timestamp) = ?) as has_target \
                            FROM movie_events \
                            GROUP BY user_id \
                            HAVING first_date = ? \
                        ) \
                    )",
                )
                .bind(target_str.as_str())
                .bind(cohort_str.as_str())
                .fetch_optional::<SingleF64>()
                .await?;
            result
                .map(|v| if v.value.is_nan() { 0.0 } else { v.value })
                .unwrap_or(0.0)
        };
        results.push(MetricResult {
            date,
            metric_name: "retention".into(),
            dimension: offset.to_string(),
            value: retention,
        });
    }

    Ok(results)
}

pub async fn write_to_clickhouse(ch: &clickhouse::Client, results: &[MetricResult]) -> Result<()> {
    #[derive(Row, Serialize)]
    struct ChMetric {
        #[serde(with = "clickhouse::serde::chrono::date")]
        date: NaiveDate,
        metric_name: String,
        dimension: String,
        value: f64,
        #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
        computed_at: chrono::DateTime<chrono::Utc>,
    }

    let now = chrono::Utc::now();
    let mut inserter = ch.insert::<ChMetric>("daily_metrics").await?;
    for r in results {
        inserter
            .write(&ChMetric {
                date: r.date,
                metric_name: r.metric_name.clone(),
                dimension: r.dimension.clone(),
                value: r.value,
                computed_at: now,
            })
            .await?;
    }
    inserter.end().await?;
    Ok(())
}
