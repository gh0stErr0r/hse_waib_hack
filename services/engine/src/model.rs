use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coordinates {
    pub lat: f64,
    pub lon: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSlot {
    pub id: String,
    pub label: String,
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub timezone: String,
    pub requires_event: bool,
    #[serde(default)]
    pub interests: Vec<String>,
    pub criteria: FilterCriteria,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilterCriteria {
    pub center: Coordinates,
    pub radius_m: f64,
    #[serde(default)]
    pub cuisines: Vec<String>,
    pub max_price_rub: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub title: String,
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub interests: Vec<String>,
    pub url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadSample {
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub value: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebSource {
    pub title: String,
    pub url: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldEvidence {
    pub field: String,
    pub source: WebSource,
    pub quote: String,
    pub checked_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroundingReport {
    pub provider: String,
    pub checked_at: String,
    pub sources: Vec<WebSource>,
    pub queries: Vec<String>,
    pub search_suggestions_html: Vec<String>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlaceCandidate {
    pub id: String,
    pub name: String,
    pub address: String,
    pub coordinates: Coordinates,
    #[serde(default)]
    pub cuisines: Vec<String>,
    pub average_price_rub: Option<u32>,
    #[serde(default)]
    pub opening_windows: Vec<TimeWindow>,
    pub website: Option<String>,
    pub menu_url: Option<String>,
    #[serde(default)]
    pub menu_with_prices: bool,
    pub relevance: f64,
    #[serde(default)]
    pub load: Vec<LoadSample>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub demo: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<FieldEvidence>,
    #[serde(default)]
    pub discovery_provider: Option<String>,
    #[serde(default)]
    pub geocoding_source: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoringBreakdown {
    pub scenario: String,
    pub relevance: f64,
    pub website_menu: f64,
    pub load: f64,
    pub event: f64,
    pub max_relevance: f64,
    pub max_website_menu: f64,
    pub max_load: f64,
    pub max_event: f64,
    pub load_factor: f64,
    pub load_estimated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoredPlace {
    pub place: PlaceCandidate,
    pub slot_id: String,
    pub score: f64,
    pub breakdown: ScoringBreakdown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankRequest {
    pub slots: Vec<TimeSlot>,
    pub candidates: Vec<PlaceCandidate>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlotResult {
    pub slot: TimeSlot,
    pub cards: Vec<ScoredPlace>,
    pub rejected: std::collections::BTreeMap<String, usize>,
    pub dynamic_buttons: Vec<Button>,
    pub text: String,
    #[serde(default)]
    pub grounding: Option<GroundingReport>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Button {
    pub label: String,
    pub value: String,
}
