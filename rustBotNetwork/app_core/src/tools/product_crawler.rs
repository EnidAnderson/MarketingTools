use crate::contracts::{ToolContract, ToolError, ToolResult, TypedTool};
use crate::tools::base_tool::BaseTool;
use async_trait::async_trait;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

const DEFAULT_BASE_URL: &str = "https://naturesdietpet.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCrawlerInput {
    pub base_url: Option<String>,
    pub max_products: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductEntry {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCrawlerOutput {
    pub base_url: String,
    pub products: Vec<ProductEntry>,
    pub competitive_insights: Vec<String>,
    pub suggested_ad_angles: Vec<String>,
}

pub struct ProductCrawlerContract;

impl ToolContract for ProductCrawlerContract {
    const NAME: &'static str = "product_crawler";
    const VERSION: &'static str = "2.0.0";
    type Input = ProductCrawlerInput;
    type Output = ProductCrawlerOutput;
}

pub struct ProductCrawlerTool;

impl ProductCrawlerTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TypedTool<ProductCrawlerContract> for ProductCrawlerTool {
    async fn run_typed(&self, input: ProductCrawlerInput) -> ToolResult<ProductCrawlerOutput> {
        let base_url = input
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let max_products = input.max_products.unwrap_or(20).clamp(1, 100);

        let base = Url::parse(&base_url).map_err(|e| {
            ToolError::validation(format!("Invalid base_url '{}': {}", base_url, e))
        })?;

        let client = reqwest::Client::new();
        let homepage_html = fetch_html(&client, base.as_str()).await?;
        let products_page = find_our_products_page(&base, &homepage_html)
            .unwrap_or_else(|| base.join("/our-products").unwrap_or_else(|_| base.clone()));

        let products_html = fetch_html(&client, products_page.as_str()).await?;
        let links = extract_product_links(&base, &products_html, max_products);

        let products = links
            .into_iter()
            .map(|url| ProductEntry {
                name: product_name_from_url(&url),
                url,
            })
            .collect::<Vec<_>>();

        let competitive_insights = build_competitive_insights(&products);
        let suggested_ad_angles = build_ad_angles(&products);

        Ok(ProductCrawlerOutput {
            base_url,
            products,
            competitive_insights,
            suggested_ad_angles,
        })
    }
}

#[async_trait]
impl BaseTool for ProductCrawlerTool {
    fn name(&self) -> &'static str {
        "product_crawler"
    }

    fn description(&self) -> &'static str {
        "Crawls product pages and returns structured competitor/product insights."
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn run(&self, input: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let typed_input: ProductCrawlerInput = serde_json::from_value(input).map_err(|e| {
            Box::new(ToolError::validation(e.to_string()))
                as Box<dyn std::error::Error + Send + Sync>
        })?;

        let output = self
            .run_typed(typed_input)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        serde_json::to_value(output).map_err(|e| {
            Box::new(ToolError::internal(e.to_string())) as Box<dyn std::error::Error + Send + Sync>
        })
    }
}

async fn fetch_html(client: &reqwest::Client, url: &str) -> ToolResult<String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| ToolError::provider(format!("Failed request to {}: {}", url, e), true))?;

    let status = response.status();
    if !status.is_success() {
        return Err(ToolError::provider(
            format!("Non-success status {} for {}", status, url),
            status.as_u16() >= 500,
        ));
    }

    response
        .text()
        .await
        .map_err(|e| ToolError::provider(format!("Failed reading body for {}: {}", url, e), true))
}

fn find_our_products_page(base: &Url, html: &str) -> Option<Url> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a").ok()?;

    for element in document.select(&selector) {
        let href = element.value().attr("href")?;
        let joined = base.join(href).ok()?;
        if joined.path().contains("our-products") {
            return Some(joined);
        }
    }

    None
}

fn extract_product_links(base: &Url, html: &str, max_products: usize) -> Vec<String> {
    let document = Html::parse_document(html);
    let selector = match Selector::parse("a") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut links = HashSet::new();
    for element in document.select(&selector) {
        let Some(href) = element.value().attr("href") else {
            continue;
        };

        let Ok(joined) = base.join(href) else {
            continue;
        };

        if joined.path().contains("/product-page/") || joined.path().starts_with("/product/") {
            links.insert(joined.to_string());
        }

        if links.len() >= max_products {
            break;
        }
    }

    links.into_iter().collect()
}

fn product_name_from_url(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|mut segments| segments.next_back().map(str::to_string))
        })
        .map(|slug| {
            slug.replace('-', " ")
                .split_whitespace()
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "Unknown Product".to_string())
}

fn build_competitive_insights(products: &[ProductEntry]) -> Vec<String> {
    let mut insights = Vec::new();
    insights.push(format!(
        "Discovered {} product pages for competitor positioning review.",
        products.len()
    ));

    if products
        .iter()
        .any(|p| p.name.to_lowercase().contains("raw"))
    {
        insights.push(
            "Raw-food messaging appears prominent; emphasize freshness and nutrition proof points."
                .to_string(),
        );
    }

    if products
        .iter()
        .any(|p| p.name.to_lowercase().contains("dental") || p.name.to_lowercase().contains("oral"))
    {
        insights.push(
            "Oral-health category present; position dental outcomes with before/after claims where compliant."
                .to_string(),
        );
    }

    insights
}

fn build_ad_angles(products: &[ProductEntry]) -> Vec<String> {
    let mut angles = Vec::new();
    angles.push(
        "Ingredient-transparency angle: spotlight named proteins and clean labels.".to_string(),
    );
    angles.push(
        "Outcome-focused angle: connect product choice to visible pet vitality and digestion."
            .to_string(),
    );

    if products.len() >= 8 {
        angles.push(
            "Assortment angle: promote bundles or routine stacks for life-stage and wellness goals.".to_string(),
        );
    }

    angles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_product_links_from_html() {
        let html = r#"
            <html><body>
              <a href="/product/raw-beef">Raw Beef</a>
              <a href="/product-page/dental-health">Dental</a>
              <a href="/blog/post">Blog</a>
            </body></html>
        "#;

        let base = Url::parse("https://example.com").expect("valid base URL");
        let links = extract_product_links(&base, html, 10);

        assert_eq!(links.len(), 2);
        assert!(links.iter().any(|l| l.contains("raw-beef")));
        assert!(links.iter().any(|l| l.contains("dental-health")));
    }

    #[test]
    fn builds_name_from_slug() {
        let name = product_name_from_url("https://example.com/product/ready-raw-beef");
        assert_eq!(name, "Ready Raw Beef");
    }
}
