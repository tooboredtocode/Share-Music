/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::fmt::Display;

#[non_exhaustive]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DiscordLocale {
    DANISH,
    GERMAN,
    ENGLISH_GB,
    ENGLISH_US,
    SPANISH,
    FRENCH,
    CROATIAN,
    ITALIAN,
    LITHUANIAN,
    HUNGARIAN,
    DUTCH,
    NORWEGIAN,
    POLISH,
    PORTUGUESE_BRAZIL,
    ROMANIAN,
    FINNISH,
    SWEDISH,
    VIETNAMESE,
    TURKISH,
    CZECH,
    GREEK,
    BULGARIAN,
    RUSSIAN,
    UKRAINIAN,
    HINDI,
    THAI,
    CHINESE_CHINA,
    JAPANESE,
    CHINESE_TAIWAN,
    KOREAN,
    NONE,
}

impl DiscordLocale {
    pub const fn to_str(&self) -> &str {
        match self {
            Self::DANISH => "da",
            Self::GERMAN => "de",
            Self::ENGLISH_GB => "en-GB",
            Self::ENGLISH_US => "en-US",
            Self::SPANISH => "es-ES",
            Self::FRENCH => "fr",
            Self::CROATIAN => "hr",
            Self::ITALIAN => "it",
            Self::LITHUANIAN => "lt",
            Self::HUNGARIAN => "hu",
            Self::DUTCH => "nl",
            Self::NORWEGIAN => "no",
            Self::POLISH => "pl",
            Self::PORTUGUESE_BRAZIL => "pt-BR",
            Self::ROMANIAN => "ro",
            Self::FINNISH => "fi",
            Self::SWEDISH => "sv-SE",
            Self::VIETNAMESE => "vi",
            Self::TURKISH => "tr",
            Self::CZECH => "cs",
            Self::GREEK => "el",
            Self::BULGARIAN => "bg",
            Self::RUSSIAN => "ru",
            Self::UKRAINIAN => "uk",
            Self::HINDI => "hi",
            Self::THAI => "th",
            Self::CHINESE_CHINA => "zh-CN",
            Self::JAPANESE => "ja",
            Self::CHINESE_TAIWAN => "zh-TW",
            Self::KOREAN => "ko",
            _ => "",
        }
    }
}

impl Display for DiscordLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl From<&str> for DiscordLocale {
    fn from(s: &str) -> Self {
        match s {
            "da" => Self::DANISH,
            "de" => Self::GERMAN,
            "en-GB" => Self::ENGLISH_GB,
            "en-US" => Self::ENGLISH_US,
            "es-ES" => Self::SPANISH,
            "fr" => Self::FRENCH,
            "hr" => Self::CROATIAN,
            "it" => Self::ITALIAN,
            "lt" => Self::LITHUANIAN,
            "hu" => Self::HUNGARIAN,
            "nl" => Self::DUTCH,
            "no" => Self::NORWEGIAN,
            "pl" => Self::POLISH,
            "pt-BR" => Self::PORTUGUESE_BRAZIL,
            "ro" => Self::ROMANIAN,
            "fi" => Self::FINNISH,
            "sv-SE" => Self::SWEDISH,
            "vi" => Self::VIETNAMESE,
            "tr" => Self::TURKISH,
            "cs" => Self::CZECH,
            "el" => Self::GREEK,
            "bg" => Self::BULGARIAN,
            "ru" => Self::RUSSIAN,
            "uk" => Self::UKRAINIAN,
            "hi" => Self::HINDI,
            "th" => Self::THAI,
            "zh-CN" => Self::CHINESE_CHINA,
            "ja" => Self::JAPANESE,
            "zh-TW" => Self::CHINESE_TAIWAN,
            "ko" => Self::KOREAN,
            _ => Self::NONE,
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
            None => Self::NONE,
        }
    }
}
