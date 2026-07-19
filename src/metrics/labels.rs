use std::borrow::Cow;

use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use twilight_model::gateway::payload::incoming::GuildCreate;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct EventLabels {
    pub shard: u32,
    pub event: Cow<'static, str>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ShardStateLabels {
    pub shard: u32,
    pub state: &'static str,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ShardLatencyLabels {
    pub shard: u32,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum GuildState {
    Available,
    Unavailable,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct GuildLabels {
    pub shard: u32,
    pub state: GuildState,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
#[allow(clippy::upper_case_acronyms)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    PATCH,
    TRACE,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ThirdPartyRateLimitLabels {
    pub method: Method,
    pub url: Cow<'static, str>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ThirdPartyLabels {
    pub method: Method,
    pub url: Cow<'static, str>,
    pub status: u16,
}

impl From<&GuildCreate> for GuildState {
    fn from(create: &GuildCreate) -> Self {
        match create {
            GuildCreate::Available(_) => GuildState::Available,
            GuildCreate::Unavailable(_) => GuildState::Unavailable,
        }
    }
}

impl From<reqwest::Method> for Method {
    fn from(value: reqwest::Method) -> Self {
        match value {
            reqwest::Method::GET => Self::GET,
            reqwest::Method::POST => Self::POST,
            reqwest::Method::PUT => Self::PUT,
            reqwest::Method::DELETE => Self::DELETE,
            reqwest::Method::HEAD => Self::HEAD,
            reqwest::Method::OPTIONS => Self::OPTIONS,
            reqwest::Method::CONNECT => Self::CONNECT,
            reqwest::Method::PATCH => Self::PATCH,
            reqwest::Method::TRACE => Self::TRACE,
            _ => panic!("Unknown method: {:?}", value),
        }
    }
}
