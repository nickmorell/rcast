use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ChaptersJson {
    chapters: Vec<ChapterRaw>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChapterRaw {
    #[serde(rename = "startTime")]
    start_time: f64,
    title: Option<String>,
    img: Option<String>,
    url: Option<String>,
    #[serde(default = "default_true")]
    toc: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub start_time: f64,
    pub title: String,
    #[allow(dead_code)]
    pub img: Option<String>,
    #[allow(dead_code)]
    pub url: Option<String>,
    pub toc: bool,
}

pub async fn fetch_chapters(url: &str) -> anyhow::Result<Vec<Chapter>> {
    let json: ChaptersJson = reqwest::get(url).await?.json().await?;
    let chapters = json
        .chapters
        .into_iter()
        .map(|c| Chapter {
            start_time: c.start_time,
            title: c.title.unwrap_or_else(|| "Untitled".to_string()),
            img: c.img,
            url: c.url,
            toc: c.toc,
        })
        .collect();
    Ok(chapters)
}

pub fn current_chapter(chapters: &[Chapter], pos_secs: f64) -> Option<&Chapter> {
    chapters.iter().rev().find(|c| c.start_time <= pos_secs)
}
