use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use metronomos_pulse::value::PulseValue;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;

use crate::constants::{GIT_BRANCH, GIT_REVISION, NAME, RUST_VERSION, VERSION};
use crate::metrics::guild_metrics::GuildMetrics;
use crate::metrics::labels::{
    EventLabels, ShardLatencyLabels, ThirdPartyLabels, ThirdPartyRateLimitLabels,
};
use crate::metrics::shard_states::ShardStates;
use crate::util::metric_utils::HasHistogramFamily;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct VersionLabels {
    pub branch: String,
    pub revision: String,
    pub rustc_version: String,
    pub version: String,
}

macro_rules! default_or {
    ($default:expr) => {
        $default
    };
    ($default:expr, $init:expr) => {
        $init
    };
}

macro_rules! register_metrics {
    ($registry:expr, $metrics:expr, $name:literal, $help:literal $(, $unit:expr )?) => {
        $registry.register($name, $help, $metrics.clone());
    };
}

macro_rules! make_metrics_store {
    (
        struct MetricsStore {
            $(
                #[name = $name:literal]
                #[help = $help:literal]
                $( #[unit = $unit:expr] )?
                $field_name:ident : $metric_type:ty $( = $init:expr )? ,
            )+
        }
    ) => {
        #[derive(Clone, PulseValue)]
        pub struct MetricsStore {
            inner: Arc<MetricsStoreInner>,
        }

        #[derive(Debug)]
        struct MetricsStoreInner {
            $(
                $field_name: $metric_type,
            )?
        }

        impl fmt::Debug for MetricsStore {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("MetricsStore")
                    $(
                        .field(stringify!($field_name), &self.inner.$field_name)
                    )?
                    .finish()
            }
        }

        impl MetricsStore {
            pub fn new() -> Self {
                let inner = Arc::new(MetricsStoreInner {
                    $(
                        $field_name: default_or!( Default::default() $(, $init )? ),
                    )?
                });

                let res = Self { inner };
                res.post_init();

                res
            }

            $(
                pub fn $field_name(&self) -> &$metric_type {
                    &self.inner.$field_name
                }
            )?

            pub(super) fn registry(&self, cluster_id: u16) -> Registry {
                let mut registry = Registry::with_prefix_and_labels(
                    "discord",
                    [
                        (Cow::from("cluster"), Cow::from(cluster_id.to_string())),
                        (Cow::from("bot"), Cow::from(NAME)),
                    ]
                    .into_iter(),
                );

                self.pre_register(&mut registry);

                $(
                    register_metrics!(&mut registry, self.$field_name(), $name, $help $(, $unit )?);
                )?

                registry
            }
        }
    };
}

make_metrics_store!(
    struct MetricsStore {
        #[name = "gateway_events"]
        #[help = "Number of events received from the gateway"]
        gateway_events: Family<EventLabels, Counter>,

        #[name = "connected_guilds"]
        #[help = "Number of guilds the bot is connected to"]
        connected_guilds: GuildMetrics = GuildMetrics::new(),

        #[name = "shard_states"]
        #[help = "States of the shards"]
        shard_states: ShardStates = ShardStates::new(),
        #[name = "shard_latencies"]
        #[help = "Latencies of the shards"]
        #[unit = Unit::Seconds]
        shard_latencies: Family<ShardLatencyLabels, Gauge<f64, AtomicU64>>,

        #[name = "3rd_party_api_request_duration_seconds"]
        #[help = "Response time for the various APIs used by the bots"]
        third_party_rate_limit: Family<ThirdPartyRateLimitLabels, Histogram>
        = Family::<ThirdPartyRateLimitLabels, Histogram>::new_with_constructor(|| {
            Histogram::new([
                0.1, 0.15, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0,
                20.0,
            ])
        }),
        #[name = "3rd_party_api_rate_limit_duration_seconds"]
        #[help = "Time spent waiting for rate limits for the various APIs used by the bots"]
        third_party_api: Family<ThirdPartyLabels, Histogram>
        = Family::<ThirdPartyLabels, Histogram>::new_with_constructor(|| {
            Histogram::new([
                0.1, 0.15, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0,
            ])
        }),

        #[name = "odesli_rate_limit_tokens"]
        #[help = "Number of tokens currently available in the Odesli rate limiter"]
        odesli_rate_limit_tokens: Gauge<u64, AtomicU64>,
    }
);

impl HasHistogramFamily<ThirdPartyRateLimitLabels> for MetricsStore {
    fn family_with_label(&self) -> &Family<ThirdPartyRateLimitLabels, Histogram> {
        self.third_party_rate_limit()
    }
}
impl HasHistogramFamily<ThirdPartyLabels> for MetricsStore {
    fn family_with_label(&self) -> &Family<ThirdPartyLabels, Histogram> {
        self.third_party_api()
    }
}

impl MetricsStore {
    fn post_init(&self) {}

    fn pre_register(&self, registry: &mut Registry) {
        let version = Family::<VersionLabels, Gauge>::default();
        version
            .get_or_create(&VersionLabels {
                branch: GIT_BRANCH.to_string(),
                revision: GIT_REVISION.to_string(),
                rustc_version: RUST_VERSION.to_string(),
                version: VERSION.to_string(),
            })
            .set(1);
        registry.register("bot_info", "Information about the bot", version);
    }
}
