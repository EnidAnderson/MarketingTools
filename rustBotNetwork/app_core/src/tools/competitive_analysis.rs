use crate::contracts::ToolError;
use crate::tools::base_tool::BaseTool;
use async_trait::async_trait;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompetitiveAnalysisInput {
    topic: String,
    max_sources: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchSource {
    title: String,
    url: String,
    snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecurringPhrase {
    phrase: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompetitiveSignal {
    signal: String,
    count: usize,
    evidence_urls: Vec<String>,
    evidence_snippets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompetitiveAnalysisOutput {
    topic: String,
    source_count: usize,
    sources: Vec<SearchSource>,
    keyword_frequency: HashMap<String, usize>,
    recurring_phrases: Vec<RecurringPhrase>,
    signals: Vec<CompetitiveSignal>,
    inferred_notes: Vec<String>,
    signal_report_markdown: String,
}

#[derive(Serialize, Deserialize)]
pub struct CompetitiveAnalysisTool {
    name: &'static str,
    description: &'static str,
}

impl CompetitiveAnalysisTool {
    pub fn new() -> Self {
        Self {
            name: "competitive_analysis",
            description: "Performs competitive analysis based on a given topic.",
        }
    }

    async fn fetch_search_results(
        topic: &str,
        max_sources: usize,
    ) -> Result<Vec<SearchSource>, ToolError> {
        let encoded: String = url::form_urlencoded::byte_serialize(topic.as_bytes()).collect();
        let url = format!("https://duckduckgo.com/html/?q={}", encoded);

        let client = reqwest::Client::new();
        let body = client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                ToolError::provider(format!("Failed to fetch live search results: {}", e), true)
            })?
            .text()
            .await
            .map_err(|e| {
                ToolError::provider(format!("Failed to parse search response body: {}", e), true)
            })?;

        let mut parsed = parse_search_results(&body);
        parsed.truncate(max_sources);

        if parsed.is_empty() {
            return Err(ToolError::provider(
                "No live market sources were found for this topic. Try a broader query."
                    .to_string(),
                true,
            ));
        }

        Ok(parsed)
    }

    fn analyze_sources(topic: &str, sources: &[SearchSource]) -> CompetitiveAnalysisOutput {
        let corpus = sources
            .iter()
            .map(|s| format!("{} {}", s.title, s.snippet))
            .collect::<Vec<_>>()
            .join(" ");

        let keyword_frequency = extract_keyword_frequency(&corpus);
        let recurring_phrases = extract_recurring_phrases(&corpus, 3);
        let signals = build_signals(sources, &keyword_frequency);

        // Notes are explicitly marked as inferred summaries derived from the raw signal table.
        let inferred_notes = derive_inferred_notes(&keyword_frequency, &recurring_phrases);

        let signal_report_markdown = build_signal_report_markdown(
            topic,
            sources,
            &keyword_frequency,
            &recurring_phrases,
            &signals,
            &inferred_notes,
        );

        CompetitiveAnalysisOutput {
            topic: topic.to_string(),
            source_count: sources.len(),
            sources: sources.to_vec(),
            keyword_frequency,
            recurring_phrases,
            signals,
            inferred_notes,
            signal_report_markdown,
        }
    }
}

#[async_trait]
impl BaseTool for CompetitiveAnalysisTool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let parsed: CompetitiveAnalysisInput = serde_json::from_value(input).map_err(|e| {
            Box::new(ToolError::validation(format!(
                "Invalid competitive_analysis input: {}",
                e
            ))) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let topic = parsed.topic.trim();
        if topic.is_empty() {
            return Err(Box::new(ToolError::validation(
                "'topic' is required and cannot be empty",
            )));
        }

        let max_sources = parsed.max_sources.unwrap_or(8).clamp(3, 20);
        let sources = Self::fetch_search_results(topic, max_sources)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let analysis = Self::analyze_sources(topic, &sources);
        serde_json::to_value(analysis).map_err(|e| {
            Box::new(ToolError::internal(e.to_string())) as Box<dyn std::error::Error + Send + Sync>
        })
    }
}

fn parse_search_results(html: &str) -> Vec<SearchSource> {
    let document = Html::parse_document(html);
    let container_selector = match Selector::parse("div.result") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let title_selector = match Selector::parse("a.result__a") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let snippet_selector = match Selector::parse("a.result__snippet") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for container in document.select(&container_selector) {
        let Some(title_node) = container.select(&title_selector).next() else {
            continue;
        };

        let title = title_node
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        let url = title_node.value().attr("href").unwrap_or("").to_string();
        let snippet = container
            .select(&snippet_selector)
            .next()
            .map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .unwrap_or_default();

        if title.is_empty() || url.is_empty() {
            continue;
        }
        if seen.contains(&url) {
            continue;
        }
        seen.insert(url.clone());

        results.push(SearchSource {
            title,
            url,
            snippet,
        });
    }

    results
}

fn extract_keyword_frequency(text: &str) -> HashMap<String, usize> {
    let lowered = text.to_lowercase();
    let keywords: [(&str, &[&str]); 10] = [
        ("price", &["price", "cost", "affordable", "value"]),
        (
            "nutrition",
            &["nutrition", "nutrient", "protein", "ingredient"],
        ),
        (
            "convenience",
            &["convenient", "easy", "quick", "ready", "simple"],
        ),
        ("trust", &["trusted", "vet", "quality", "safe", "certified"]),
        (
            "health",
            &["digest", "energy", "coat", "health", "wellness"],
        ),
        (
            "subscription",
            &["subscription", "subscribe", "monthly", "delivery"],
        ),
        ("raw", &["raw", "freeze-dried", "freeze dried", "fresh"]),
        ("reviews", &["review", "rating", "testimonial", "feedback"]),
        ("natural", &["natural", "clean", "whole"]),
        (
            "breed-specific",
            &["puppy", "senior", "small breed", "large breed"],
        ),
    ];

    let mut counts = HashMap::new();
    for (label, terms) in keywords {
        let c = terms
            .iter()
            .map(|t| lowered.matches(t).count())
            .sum::<usize>();
        if c > 0 {
            counts.insert(label.to_string(), c);
        }
    }
    counts
}

fn extract_recurring_phrases(text: &str, min_count: usize) -> Vec<RecurringPhrase> {
    let stopwords: HashSet<&str> = [
        "the", "a", "and", "for", "to", "of", "in", "on", "with", "from", "is", "are", "your",
        "dog", "food",
    ]
    .into_iter()
    .collect();

    let tokens = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| w.len() > 2 && !stopwords.contains(*w))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let mut counts: HashMap<String, usize> = HashMap::new();
    for window in tokens.windows(2) {
        let phrase = format!("{} {}", window[0], window[1]);
        *counts.entry(phrase).or_insert(0) += 1;
    }

    let mut phrases = counts
        .into_iter()
        .filter(|(_, c)| *c >= min_count)
        .map(|(phrase, count)| RecurringPhrase { phrase, count })
        .collect::<Vec<_>>();

    phrases.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.phrase.cmp(&b.phrase)));
    phrases.truncate(12);
    phrases
}

fn build_signals(
    sources: &[SearchSource],
    keyword_frequency: &HashMap<String, usize>,
) -> Vec<CompetitiveSignal> {
    let signal_terms: [(&str, &[&str]); 10] = [
        ("price", &["price", "cost", "affordable", "value"]),
        (
            "nutrition",
            &["nutrition", "nutrient", "protein", "ingredient"],
        ),
        (
            "convenience",
            &["convenient", "easy", "quick", "ready", "simple"],
        ),
        ("trust", &["trusted", "vet", "quality", "safe", "certified"]),
        (
            "health",
            &["digest", "energy", "coat", "health", "wellness"],
        ),
        (
            "subscription",
            &["subscription", "subscribe", "monthly", "delivery"],
        ),
        ("raw", &["raw", "freeze-dried", "freeze dried", "fresh"]),
        ("reviews", &["review", "rating", "testimonial", "feedback"]),
        ("natural", &["natural", "clean", "whole"]),
        (
            "breed-specific",
            &["puppy", "senior", "small breed", "large breed"],
        ),
    ];

    let mut signals = Vec::new();

    for (signal, terms) in signal_terms {
        let count = *keyword_frequency.get(signal).unwrap_or(&0);
        if count == 0 {
            continue;
        }

        let mut evidence_urls = Vec::new();
        let mut evidence_snippets = Vec::new();

        for src in sources {
            let lowered = format!(
                "{} {}",
                src.title.to_lowercase(),
                src.snippet.to_lowercase()
            );
            if terms.iter().any(|t| lowered.contains(t)) {
                evidence_urls.push(src.url.clone());
                if !src.snippet.trim().is_empty() {
                    evidence_snippets.push(src.snippet.clone());
                } else {
                    evidence_snippets.push(src.title.clone());
                }
            }
        }

        evidence_urls.sort();
        evidence_urls.dedup();
        evidence_snippets.truncate(5);

        signals.push(CompetitiveSignal {
            signal: signal.to_string(),
            count,
            evidence_urls,
            evidence_snippets,
        });
    }

    signals.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.signal.cmp(&b.signal)));
    signals
}

fn derive_inferred_notes(
    keyword_frequency: &HashMap<String, usize>,
    recurring_phrases: &[RecurringPhrase],
) -> Vec<String> {
    let mut notes = Vec::new();

    if let Some(c) = keyword_frequency.get("raw") {
        if *c >= 3 {
            notes.push(
                "Inferred: raw-positioning language appears repeatedly in the sampled market copy."
                    .to_string(),
            );
        }
    }

    if let Some(top) = recurring_phrases.first() {
        notes.push(format!(
            "Inferred: the most repeated phrase pattern in sampled snippets is '{}' ({} hits).",
            top.phrase, top.count
        ));
    }

    if notes.is_empty() {
        notes.push(
            "Inferred: no dominant repeated narrative emerged from current sampled sources."
                .to_string(),
        );
    }

    notes
}

fn build_signal_report_markdown(
    topic: &str,
    sources: &[SearchSource],
    keyword_frequency: &HashMap<String, usize>,
    recurring_phrases: &[RecurringPhrase],
    signals: &[CompetitiveSignal],
    inferred_notes: &[String],
) -> String {
    let keywords_md = if keyword_frequency.is_empty() {
        "- none".to_string()
    } else {
        let mut items = keyword_frequency
            .iter()
            .map(|(k, v)| format!("- {}: {}", k, v))
            .collect::<Vec<_>>();
        items.sort();
        items.join("\n")
    };

    let phrases_md = if recurring_phrases.is_empty() {
        "- none".to_string()
    } else {
        recurring_phrases
            .iter()
            .map(|p| format!("- {} ({})", p.phrase, p.count))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let signals_md = if signals.is_empty() {
        "- none".to_string()
    } else {
        signals
            .iter()
            .map(|s| {
                format!(
                    "- {} ({} mentions)\n  - evidence urls: {}\n  - sample snippets: {}",
                    s.signal,
                    s.count,
                    s.evidence_urls.join(", "),
                    s.evidence_snippets.join(" | ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let sources_md = sources
        .iter()
        .map(|s| format!("- {} ({})\n  - {}", s.title, s.url, s.snippet))
        .collect::<Vec<_>>()
        .join("\n");

    let notes_md = inferred_notes
        .iter()
        .map(|n| format!("- {}", n))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "# Competitive Signal Report: {}\n\n## Source Coverage\n- sources: {}\n\n## Raw Keyword Frequency\n{}\n\n## Recurring Phrases\n{}\n\n## Signals With Evidence\n{}\n\n## Sources\n{}\n\n## Inferred Notes (derived from above signals)\n{}",
        topic,
        sources.len(),
        keywords_md,
        phrases_md,
        signals_md,
        if sources_md.is_empty() { "- none" } else { &sources_md },
        notes_md,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_ddg_result_html() {
        let html = r#"
        <div class='result'>
            <a class='result__a' href='https://example.com/a'>Brand A Freeze Dried Raw</a>
            <a class='result__snippet'>Affordable raw nutrition with simple prep.</a>
        </div>
        <div class='result'>
            <a class='result__a' href='https://example.com/b'>Brand B Dog Food Review</a>
            <a class='result__snippet'>Trusted quality and ingredient transparency.</a>
        </div>
        "#;

        let parsed = parse_search_results(html);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].url, "https://example.com/a");
    }

    #[test]
    fn builds_signal_evidence() {
        let sources = vec![
            SearchSource {
                title: "A".to_string(),
                url: "https://a.com".to_string(),
                snippet: "Affordable raw nutrition".to_string(),
            },
            SearchSource {
                title: "B".to_string(),
                url: "https://b.com".to_string(),
                snippet: "Trusted quality and easy prep".to_string(),
            },
        ];
        let corpus = sources
            .iter()
            .map(|s| format!("{} {}", s.title, s.snippet))
            .collect::<Vec<_>>()
            .join(" ");

        let freq = extract_keyword_frequency(&corpus);
        let signals = build_signals(&sources, &freq);

        assert!(signals.iter().any(|s| s.signal == "raw"));
        assert!(signals.iter().any(|s| s.signal == "trust"));
    }

    #[tokio::test]
    async fn rejects_empty_topic() {
        let tool = CompetitiveAnalysisTool::new();
        let err = tool
            .run(json!({"topic": "   "}))
            .await
            .expect_err("empty topic must fail");
        assert!(err.to_string().contains("cannot be empty"));
    }
}
