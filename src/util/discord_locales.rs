/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::fmt::Display;

#[non_exhaustive]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DiscordLocale {
    Danish,
    German,
    EnglishGB,
    EnglishUS,
    Spanish,
    French,
    Croatian,
    Italian,
    Lithuanian,
    Hungarian,
    Dutch,
    Norwegian,
    Polish,
    PortugueseBrazil,
    Romanian,
    Finnish,
    Swedish,
    Vietnamese,
    Turkish,
    Czech,
    Greek,
    Bulgarian,
    Russian,
    Ukrainian,
    Hindi,
    Thai,
    ChineseChina,
    Japanese,
    ChineseTaiwan,
    Korean,
    None,
}

impl DiscordLocale {
    #[allow(clippy::wrong_self_convention)]
    pub const fn to_str(&self) -> &str {
        match self {
            Self::Danish => "da",
            Self::German => "de",
            Self::EnglishGB => "en-GB",
            Self::EnglishUS => "en-US",
            Self::Spanish => "es-ES",
            Self::French => "fr",
            Self::Croatian => "hr",
            Self::Italian => "it",
            Self::Lithuanian => "lt",
            Self::Hungarian => "hu",
            Self::Dutch => "nl",
            Self::Norwegian => "no",
            Self::Polish => "pl",
            Self::PortugueseBrazil => "pt-BR",
            Self::Romanian => "ro",
            Self::Finnish => "fi",
            Self::Swedish => "sv-SE",
            Self::Vietnamese => "vi",
            Self::Turkish => "tr",
            Self::Czech => "cs",
            Self::Greek => "el",
            Self::Bulgarian => "bg",
            Self::Russian => "ru",
            Self::Ukrainian => "uk",
            Self::Hindi => "hi",
            Self::Thai => "th",
            Self::ChineseChina => "zh-CN",
            Self::Japanese => "ja",
            Self::ChineseTaiwan => "zh-TW",
            Self::Korean => "ko",
            Self::None => "",
        }
    }
}

impl Display for DiscordLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl From<&str> for DiscordLocale {
    fn from(s: &str) -> Self {
        match s {
            "da" => Self::Danish,
            "de" => Self::German,
            "en-GB" => Self::EnglishGB,
            "en-US" => Self::EnglishUS,
            "es-ES" => Self::Spanish,
            "fr" => Self::French,
            "hr" => Self::Croatian,
            "it" => Self::Italian,
            "lt" => Self::Lithuanian,
            "hu" => Self::Hungarian,
            "nl" => Self::Dutch,
            "no" => Self::Norwegian,
            "pl" => Self::Polish,
            "pt-BR" => Self::PortugueseBrazil,
            "ro" => Self::Romanian,
            "fi" => Self::Finnish,
            "sv-SE" => Self::Swedish,
            "vi" => Self::Vietnamese,
            "tr" => Self::Turkish,
            "cs" => Self::Czech,
            "el" => Self::Greek,
            "bg" => Self::Bulgarian,
            "ru" => Self::Russian,
            "uk" => Self::Ukrainian,
            "hi" => Self::Hindi,
            "th" => Self::Thai,
            "zh-CN" => Self::ChineseChina,
            "ja" => Self::Japanese,
            "zh-TW" => Self::ChineseTaiwan,
            "ko" => Self::Korean,
            _ => Self::None,
        }
    }
}

impl From<String> for DiscordLocale {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<&Option<String>> for DiscordLocale {
    fn from(o: &Option<String>) -> Self {
        match o {
            Some(s) => Self::from(s.as_str()),
            None => Self::None,
        }
    }
}
