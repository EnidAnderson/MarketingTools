use super::base_tool::BaseTool;
use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use serde_json::Value;
use std::error::Error;

pub struct EventCalendarTool;

impl EventCalendarTool {
    pub fn new() -> Self {
        EventCalendarTool
    }

    // Modified to take a `NaiveDate` for consistency in testing
    fn _generate_mock_events(base_date: NaiveDate) -> Vec<Value> {
        let events_data = vec![
            (
                "e001",
                "Marketing Strategy Meeting",
                base_date - Duration::days(2),
                "Review Q1 marketing strategy.",
            ),
            (
                "e002",
                "Product Launch Planning",
                base_date,
                "Product launch planning and finalization for new dog food line.",
            ), // Modified description
            (
                "e003",
                "Team Brainstorm: Social Media",
                base_date + Duration::days(1),
                "Brainstorming session for upcoming social media campaign.",
            ),
            (
                "e004",
                "Content Calendar Review",
                base_date + Duration::days(5),
                "Review blog posts and video content plan.",
            ),
            (
                "e005",
                "Client Meeting: PetCo",
                base_date + Duration::days(10),
                "Discuss Q2 partnership with PetCo.",
            ),
            (
                "e006",
                "Budget Approval",
                base_date + Duration::days(1),
                "Financial review and budget approval for next quarter.",
            ),
            (
                "e007",
                "SEO Workshop",
                base_date + Duration::days(3),
                "Workshop on latest SEO techniques for pet niche.",
            ),
        ];

        events_data
            .into_iter()
            .map(|(id, title, date, description)| {
                serde_json::json!({
                    "id": id,
                    "title": title,
                    "date": date.format("%Y-%m-%d").to_string(), // Format as "YYYY-MM-DD"
                    "description": description,
                })
            })
            .collect()
    }
}
#[async_trait]
impl BaseTool for EventCalendarTool {
    fn name(&self) -> &'static str {
        "EventCalendarTool"
    }

    fn description(&self) -> &'static str {
        "Retrieves mock calendar events for a given date range, optionally filtered by keywords."
    }

    fn is_available(&self) -> bool {
        true // Conceptual mock tool, always available
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let start_date_str = input["start_date"].as_str();
        let end_date_str = input["end_date"].as_str();
        let keywords: Vec<String> = input["keywords"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
            .collect();

        if start_date_str.is_none() || end_date_str.is_none() {
            return Ok(serde_json::json!({
                "status": "error",
                "message": "start_date and end_date are required."
            }));
        }

        let start_date_naive = NaiveDate::parse_from_str(start_date_str.unwrap(), "%Y-%m-%d")?;
        let end_date_naive = NaiveDate::parse_from_str(end_date_str.unwrap(), "%Y-%m-%d")?;

        // Use a consistent 'today' for generating mock events in run()
        // This makes `run` deterministic for a given input date range relative to its mock data.
        let run_base_date = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap(); // Fixed date for consistency
        let mock_events = EventCalendarTool::_generate_mock_events(run_base_date);

        let filtered_events: Vec<Value> = mock_events
            .into_iter()
            .filter(|event| {
                let event_date_str = event["date"].as_str().unwrap_or_default();
                let event_date_naive = NaiveDate::parse_from_str(event_date_str, "%Y-%m-%d")
                    .unwrap_or_else(|_| NaiveDate::MIN);

                // Filter by date range
                if !(start_date_naive <= event_date_naive && event_date_naive <= end_date_naive) {
                    return false;
                }

                // Filter by keywords if any are provided
                if !keywords.is_empty() {
                    let description = event["description"]
                        .as_str()
                        .unwrap_or_default()
                        .to_lowercase();
                    if !keywords.iter().any(|k| description.contains(k)) {
                        return false;
                    }
                }
                true
            })
            .collect();

        Ok(serde_json::json!({
            "status": "success",
            "events": filtered_events
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    // Use a fixed base date for all tests to ensure consistent mock event generation
    const TEST_BASE_DATE: NaiveDate = NaiveDate::from_ymd_opt(2026, 1, 30).unwrap();

    #[test]
    fn test_is_available() {
        let tool = EventCalendarTool::new();
        assert!(tool.is_available());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_missing_dates() {
        // Added async
        let tool = EventCalendarTool::new();
        let input = json!({});
        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "error");
        assert!(result["message"]
            .as_str()
            .unwrap()
            .contains("start_date and end_date are required"));
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_date_range_filtering() {
        // Added async
        let tool = EventCalendarTool::new();
        let today_naive = TEST_BASE_DATE;
        let tomorrow_naive = today_naive + Duration::days(1);
        let _day_after_naive = today_naive + Duration::days(2); // Prefixed with _ to silence warning

        // Events that should be within range (relative to TEST_BASE_DATE): e002 (today), e003 (tomorrow), e006 (tomorrow)
        let input = json!({
            "start_date": today_naive.format("%Y-%m-%d").to_string(),
            "end_date": tomorrow_naive.format("%Y-%m-%d").to_string(),
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        let events = result["events"].as_array().unwrap();
        assert_eq!(events.len(), 3); // e002, e003, e006

        let event_ids: Vec<String> = events
            .iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();
        assert!(event_ids.contains(&"e002".to_string()));
        assert!(event_ids.contains(&"e003".to_string()));
        assert!(event_ids.contains(&"e006".to_string()));

        // Test with a range that should yield no events
        let far_future = today_naive + Duration::days(100);
        let input_empty = json!({
            "start_date": far_future.format("%Y-%m-%d").to_string(),
            "end_date": (far_future + Duration::days(1)).format("%Y-%m-%d").to_string(),
        });
        let result_empty = tool.run(input_empty).await.unwrap(); // Added .await
        let events_empty = result_empty["events"].as_array().unwrap();
        assert!(events_empty.is_empty());
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_keyword_filtering() {
        // Added async
        let tool = EventCalendarTool::new();
        let ten_days_ago_naive = TEST_BASE_DATE - Duration::days(10);
        let ten_days_future_naive = TEST_BASE_DATE + Duration::days(10);

        // Filter for "planning" keyword
        let input_planning = json!({
            "start_date": ten_days_ago_naive.format("%Y-%m-%d").to_string(),
            "end_date": ten_days_future_naive.format("%Y-%m-%d").to_string(),
            "keywords": ["planning"]
        });
        let result_planning = tool.run(input_planning).await.unwrap(); // Added .await
        let events_planning = result_planning["events"].as_array().unwrap();
        assert_eq!(events_planning.len(), 1);
        assert_eq!(events_planning[0]["id"], "e002"); // Product Launch Planning

        // Filter for "review" keyword (case-insensitive)
        let input_review = json!({
            "start_date": ten_days_ago_naive.format("%Y-%m-%d").to_string(),
            "end_date": ten_days_future_naive.format("%Y-%m-%d").to_string(),
            "keywords": ["Review"]
        });
        let result_review = tool.run(input_review).await.unwrap(); // Added .await
        let events_review = result_review["events"].as_array().unwrap();
        assert_eq!(events_review.len(), 3); // Changed from 2 to 3
        let event_ids: Vec<String> = events_review
            .iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();
        assert!(event_ids.contains(&"e001".to_string()));
        assert!(event_ids.contains(&"e004".to_string()));
        assert!(event_ids.contains(&"e006".to_string())); // Added assertion for e006
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_keyword_and_date_filtering() {
        // Added async
        let tool = EventCalendarTool::new();
        let today_naive = TEST_BASE_DATE;
        let tomorrow_naive = today_naive + Duration::days(1);

        // Filter for "social" keyword within today and tomorrow
        let input = json!({
            "start_date": today_naive.format("%Y-%m-%d").to_string(),
            "end_date": tomorrow_naive.format("%Y-%m-%d").to_string(),
            "keywords": ["social"]
        });

        let result = tool.run(input).await.unwrap(); // Added .await
        assert_eq!(result["status"], "success");
        let events = result["events"].as_array().unwrap();
        assert_eq!(events.len(), 1); // e003: Team Brainstorm: Social Media
        assert_eq!(events[0]["id"], "e003");
    }

    #[tokio::test] // Changed to tokio::test
    async fn test_run_invalid_date_format() {
        // Added async
        let tool = EventCalendarTool::new();
        let input = json!({
            "start_date": "2023/10/26", // Invalid format
            "end_date": "2023-10-27"
        });
        let result = tool.run(input).await; // Added .await
        assert!(result.is_err()); // Expect an error from date parsing
    }
}
