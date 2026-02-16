// rustBotNetwork/app_core/src/data_models/dashboard.rs

use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;

/// Enum for different types of visualizations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    Bar,
    Line,
    Pie,
    Doughnut,
    Table,
    Summary,
}

/// Enum for predefined date range presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateRangePreset {
    Last7Days,
    Last30Days,
    ThisMonth,
    LastMonth,
    ThisYear,
    LastYear,
}

/// Defines a single widget (chart or table) within a dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub r#type: WidgetType, // `type` is a Rust keyword
    pub title: String,
    pub data_source: String, // e.g., "total_metrics", "campaign_data"
    pub metrics: Vec<String>, // Metrics to display/chart
    pub dimension: Option<String>, // Dimension to group by for charts
    pub limit: Option<u32>, // Optional limit on number of items
    pub sort_by: Option<String>, // Metric to sort by
    pub sort_order: Option<String>, // "asc" or "desc"
    pub chart_options: Option<Value>, // Flexible JSON for Chart.js options
}

/// Defines the overall structure for a custom analytics dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub dashboard_name: String,
    pub description: Option<String>,
    pub date_range_preset: Option<DateRangePreset>,
    pub start_date: Option<String>, // YYYY-MM-DD
    pub end_date: Option<String>,   // YYYY-MM-DD
    pub filters: Option<HashMap<String, String>>, // Global filters
    pub widgets: Vec<WidgetConfig>,
}

// --- Data Structures for Frontend Rendering ---

/// Data for rendering a single chart or table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<ChartDataset>,
}

/// A dataset within a chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataset {
    pub label: String,
    pub data: Vec<f64>,
    #[serde(flatten)]
    pub options: Option<HashMap<String, Value>>, // Flexible options for Chart.js dataset
}

/// Data for rendering a single widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetRenderData {
    pub widget_id: String,
    pub r#type: WidgetType,
    pub title: String,
    pub chart_data: Option<ChartData>, // For charts
    pub table_data: Option<Vec<HashMap<String, Value>>>, // For tables
    pub summary_data: Option<HashMap<String, Value>>, // For summary widgets
    pub chart_options: Option<Value>, // Chart.js specific options
}

/// The complete data structure returned to the frontend for rendering a dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardRenderData {
    pub dashboard_name: String,
    pub date_range: String, // Actual date range used
    pub widgets: Vec<WidgetRenderData>,
}
