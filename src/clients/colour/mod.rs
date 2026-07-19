/*
 * Copyright (c) 2021-2026 tooboredtocode
 * All Rights Reserved
 */

use std::borrow::Cow;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use image::DynamicImage;
use image::imageops::FilterType;
use metronomos_pulse::value::{ArcValue, PulseValue};
use tracing::{Instrument, debug, debug_span, instrument};
use url::Host;

use crate::color_config::ColorConfig;
use crate::constants::colour_consts;
use crate::metrics::MetricsStore;
use crate::metrics::labels::{Method, ThirdPartyLabels};
use crate::util::EmptyResult;
use crate::util::error::expect_warn;
use crate::util::metric_utils::{HasHistogramFamilyExt, TimeFutureExt, UnpackErr};

mod hsl_pixel;
mod pixel_group;
mod rgb_pixel;

use hsl_pixel::HslPixel;
use pixel_group::PixelGroup;
pub use rgb_pixel::RGBPixel;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct OptionsOverride {
    pub brightest_percent: Option<f32>,
    pub percent_factor: Option<f32>,
    pub saturation_factor: Option<f32>,
    pub luminosity_factor: Option<f32>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct PopulatedOptions {
    pub brightest_percent: f32,
    pub percent_factor: f32,
    pub saturation_factor: f32,
    pub luminosity_factor: f32,
}

impl PopulatedOptions {
    pub fn new(defaults: ColorConfig, overrides: OptionsOverride) -> Self {
        Self {
            brightest_percent: overrides
                .brightest_percent
                .unwrap_or(defaults.brightest_percent),
            percent_factor: overrides.percent_factor.unwrap_or(defaults.percent_factor),
            saturation_factor: overrides
                .saturation_factor
                .unwrap_or(defaults.saturation_factor),
            luminosity_factor: overrides
                .luminosity_factor
                .unwrap_or(defaults.luminosity_factor),
        }
    }
}

#[derive(Clone, PulseValue)]
pub struct ImageClient {
    inner: Arc<ImageClientInner>,
}

struct ImageClientInner {
    client: reqwest::Client,
    options: ColorConfig,
    metrics: MetricsStore,
}

impl fmt::Debug for ImageClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageClient")
            .field("client", &self.inner.client)
            .finish()
    }
}

impl ImageClient {
    pub fn init(
        client: reqwest::Client,
        options: ArcValue<ColorConfig>,
        metrics: MetricsStore,
    ) -> Self {
        let inner = Arc::new(ImageClientInner {
            client,
            options: options.deref().clone(),
            metrics,
        });

        Self { inner }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn get_dominant_colour(
        &self,
        image: &DynamicImage,
        options_override: OptionsOverride,
    ) -> EmptyResult<RGBPixel> {
        let options = PopulatedOptions::new(self.inner.options, options_override);

        let num_pixels = image.height() * image.width();
        let mut groups: Vec<PixelGroup> = image.to_rgb8().pixels().map(RGBPixel::from).collect();

        groups = {
            let mut res = Vec::new();
            let mut count = 0.0;

            groups.sort_by(|a, b| b.group_luminosity.total_cmp(&a.group_luminosity));

            for g in groups {
                count += g.percentage(num_pixels);

                res.push(g);

                if count > options.brightest_percent {
                    break;
                }
            }

            res
        };

        groups
            .iter()
            .max_by(|a, b| {
                a.dom_val(num_pixels, options)
                    .total_cmp(&b.dom_val(num_pixels, options))
            })
            .ok_or(())?
            .most_common_colour()
            .ok_or(())
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn get_dominant_colour_from_url(
        &self,
        url: &String,
        options_override: OptionsOverride,
    ) -> EmptyResult<RGBPixel> {
        let image = self.fetch_image(url).await?;
        self.get_dominant_colour(&image, options_override)
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn fetch_image(&self, url: &String) -> EmptyResult<DynamicImage> {
        debug!(url, "Fetching image");

        let req = self
            .inner
            .client
            .get(url)
            .build()
            .map_err(expect_warn!("Failed to build thumbnail request"))?;

        let metrics_url = format!(
            "{}://{}",
            req.url().scheme(),
            req.url().host().unwrap_or(Host::Domain("unknown.host"))
        );

        let (resp, diff) = self
            .inner
            .client
            .execute(req)
            .instrument(debug_span!("http_request"))
            .time()
            .await
            .unpack_err()
            .map_err(expect_warn!("Failed to fetch thumbnail"))?;

        self.inner.metrics.observe_duration(
            ThirdPartyLabels {
                method: Method::GET,
                url: Cow::from(metrics_url),
                status: resp.status().into(),
            },
            diff,
        );

        const EMPTY: &[u8] = &[];
        let bytes = resp
            .bytes()
            .await
            .map_err(expect_warn!("Failed to read thumbnail bytes"));
        let mut img =
            image::load_from_memory(bytes.as_deref().unwrap_or(EMPTY)).map_err(expect_warn!(
                "Failed to parse image, url may have pointed to a file that wasn't an image"
            ))?;

        if (colour_consts::MAX_IMAGE_SIZE < img.width())
            | (colour_consts::MAX_IMAGE_SIZE < img.height())
        {
            img = img.resize(
                colour_consts::MAX_IMAGE_SIZE,
                colour_consts::MAX_IMAGE_SIZE,
                FilterType::Nearest,
            );
        }

        debug!("Successfully parsed image");
        Ok(img)
    }
}
