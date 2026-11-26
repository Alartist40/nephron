//! Perplexica-style web search client (SearXNG JSON + DuckDuckGo HTML fallback).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Clone)]
pub struct WebSearcher {
    searxng_url: String,
    http: reqwest::Client,
}

impl Default for WebSearcher {
    fn default() -> Self {
        let url = std::env::var("NEPHRON_SEARXNG_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:4000".into());
        Self::new(url)
    }
}

impl WebSearcher {
    pub fn new(searxng_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .user_agent("Nephron/0.1")
            .build()
            .unwrap_or_default();
        Self {
            searxng_url: searxng_url.into().trim_end_matches('/').to_string(),
            http,
        }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        if let Ok(hits) = self.search_searxng(query, limit).await {
            if !hits.is_empty() {
                return Ok(hits);
            }
        }
        self.search_ddg(query, limit).await
    }

    async fn search_searxng(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let resp = self
            .http
            .get(format!("{}/search", self.searxng_url))
            .query(&[("q", query), ("format", "json")])
            .send()
            .await?;
        let val: serde_json::Value = resp.json().await?;
        let arr = val.get("results").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        Ok(arr
            .into_iter()
            .take(limit)
            .filter_map(|r| {
                Some(SearchResult {
                    title: r.get("title")?.as_str()?.to_string(),
                    url: r.get("url")?.as_str()?.to_string(),
                    snippet: r.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                })
            })
            .collect())
    }

    async fn search_ddg(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let html = self
            .http
            .post("https://html.duckduckgo.com/html/")
            .form(&[("q", query)])
            .send()
            .await?
            .text()
            .await?;
        Ok(parse_ddg_html(&html, limit))
    }
}

pub fn parse_ddg_html(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for chunk in html.split("class=\"result__a\"").skip(1) {
        if results.len() >= limit {
            break;
        }
        let url = extract_between(chunk, "href=\"", "\"").unwrap_or_default();
        let title = strip_tags(extract_between(chunk, ">", "</a>").unwrap_or_default());
        let snippet = chunk
            .split("class=\"result__snippet\"")
            .nth(1)
            .and_then(|s| extract_between(s, ">", "</a>").or_else(|| extract_between(s, ">", "</span>")))
            .map(strip_tags)
            .unwrap_or_default();
        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResult { title, url: url.to_string(), snippet });
        }
    }
    results
}

pub fn format_citations_prompt(query: &str, results: &[SearchResult]) -> String {
    let mut out = format!("Web search results for \"{query}\":\n");
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!("[{}] {} ({})\n{}\n", i + 1, r.title, r.url, r.snippet));
    }
    out.push_str("\nAnswer concisely using [1], [2] citations where relevant.");
    out
}

fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let a = s.find(start)? + start.len();
    let b = s[a..].find(end)? + a;
    Some(&s[a..b])
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&quot;", "\"").replace("&#x27;", "'").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ddg_and_formats_citations() {
        let html = r#"<a class="result__a" href="https://rust-lang.org">Rust <b>Lang</b></a><a class="result__snippet">Fast systems language</a>"#;
        let hits = parse_ddg_html(html, 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Rust Lang");
        assert_eq!(hits[0].url, "https://rust-lang.org");
        let prompt = format_citations_prompt("rust", &hits);
        assert!(prompt.contains("[1] Rust Lang"));
    }
}
