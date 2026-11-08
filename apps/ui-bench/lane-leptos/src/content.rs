//! Shared page content: the same `content/site.json` the Bun lane imports.
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Site {
    pub brand: String,
    pub nav: Vec<Nav>,
    pub hero: Hero,
    pub stats: Vec<Stat>,
    pub cards: Vec<CardItem>,
    pub faq: Vec<Faq>,
    pub gallery: Gallery,
    pub legal: Legal,
    pub footer: String,
}

#[derive(Deserialize)]
pub struct Nav {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Deserialize)]
pub struct Hero {
    pub eyebrow: String,
    pub title: String,
    pub lead: String,
    pub primary: String,
    pub secondary: String,
}

#[derive(Deserialize)]
pub struct Stat {
    pub label: String,
    pub value: u32,
}

#[derive(Deserialize)]
pub struct CardItem {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub cta: String,
}

#[derive(Deserialize)]
pub struct Faq {
    pub q: String,
    pub a: String,
}

#[derive(Deserialize)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct DialogText {
    pub trigger: String,
    pub title: String,
    pub body: String,
    pub confirm: String,
    pub cancel: String,
}

#[derive(Deserialize)]
pub struct Person {
    pub initials: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Row {
    pub name: String,
    pub lane: String,
    pub status: String,
    pub score: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gallery {
    pub title: String,
    pub lead: String,
    pub buttons: Vec<String>,
    pub badges: Vec<String>,
    pub select_label: String,
    pub options: Vec<String>,
    pub tabs: Vec<Tab>,
    pub dialog: DialogText,
    pub progress: u32,
    pub people: Vec<Person>,
    pub table: Vec<Row>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Legal {
    pub title: String,
    pub updated: String,
    pub intro: String,
    pub toc_label: String,
    pub sections: usize,
    pub topics: Vec<String>,
    pub paragraphs: Vec<String>,
}

pub struct LegalSection {
    pub id: String,
    pub title: String,
    pub paragraphs: Vec<String>,
}

impl Site {
    pub fn load() -> Site {
        serde_json::from_str(include_str!("../../content/site.json")).expect("content/site.json")
    }

    /// Same expansion rule as `shared/content.ts::legalSections`.
    pub fn legal_sections(&self) -> Vec<LegalSection> {
        let l = &self.legal;
        (0..l.sections)
            .map(|i| LegalSection {
                id: format!("art-{}", i + 1),
                title: format!(
                    "{}. {}{}",
                    i + 1,
                    l.topics[i % l.topics.len()],
                    if i >= l.topics.len() { " (suite)" } else { "" }
                ),
                paragraphs: (0..3)
                    .map(|k| l.paragraphs[(i * 3 + k) % l.paragraphs.len()].clone())
                    .collect(),
            })
            .collect()
    }
}

pub fn href(kit: &str, path: &str) -> String {
    if path.is_empty() {
        format!("/{kit}/")
    } else {
        format!("/{kit}/{path}")
    }
}
