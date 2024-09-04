/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use image::imageops::FilterType;
use image::DynamicImage;
use std::borrow::Cow;
use std::time::Instant;
use tracing::{debug, debug_span, instrument, Instrument};
use url::Host;

use hsl_pixel::HslPixel;
use pixel_group::PixelGroup;
pub use rgb_pixel::RGBPixel;

use crate::constants::colour_consts;
use crate::context::metrics::{Method, ThirdPartyLabels};
use crate::context::Ctx;
use crate::util::error::Expectable;

mod hsl_pixel;
mod pixel_group;
mod rgb_pixel;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Options {
    pub brightest_percent: Option<f32>,
    pub percent_factor: Option<f32>,
    pub saturation_factor: Option<f32>,
    pub luminosity_factor: Option<f32>,
}

impl Options {
    fn populate(self, ctx: &Ctx) -> PopulatedOptions {
        let cfg = &ctx.cfg.colour;

        PopulatedOptions {
            brightest_percent: self.brightest_percent.unwrap_or(cfg.brightest_percent),
            percent_factor: self.percent_factor.unwrap_or(cfg.percent_factor),
            saturation_factor: self.saturation_factor.unwrap_or(cfg.saturation_factor),
            luminosity_factor: self.luminosity_factor.unwrap_or(cfg.luminosity_factor),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct PopulatedOptions {
    pub brightest_percent: f32,
    pub percent_factor: f32,
    pub saturation_factor: f32,
    pub luminosity_factor: f32,
}

#[instrument(level = "debug", skip_all)]
pub async fn get_dominant_colour(
    url: &String,
    context: &Ctx,
    options: Options,
) -> Option<RGBPixel> {
    let options = options.populate(context);

    let img = fetch_image(url, context).await?;

    let num_pixels = img.height() * img.width();
    let mut groups: Vec<PixelGroup> = img.to_rgb8().pixels().map(RGBPixel::from).collect();

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
        })?
        .most_common_colour()
}

#[instrument(level = "debug", skip_all)]
async fn fetch_image(url: &String, context: &Ctx) -> Option<DynamicImage> {
    debug!(url, "Fetching image");

    let req = context
        .http_client
        .get(url)
        .build()
        .warn_with("Failed to build thumbnail request")?;

    let metrics_url = format!(
        "{}://{}",
        req.url().scheme(),
        req.url().host().unwrap_or(Host::Domain("unknown.host"))
    );

    let now = Instant::now();
    let resp = context
        .http_client
        .execute(req)
        .instrument(debug_span!("http_request"))
        .await
        .warn_with("Failed to fetch thumbnail")?;
    let diff = now.elapsed();

    context
        .metrics
        .third_party_api
        .get_or_create(&ThirdPartyLabels {
            method: Method::GET,
            url: Cow::from(metrics_url),
            status: resp.status().into(),
        })
        .observe(diff.as_secs_f64());

    const EMPTY: &[u8] = &[];
    let bytes = resp
        .bytes()
        .await
        .warn_with("Failed to read thumbnail bytes");
    let mut img = image::load_from_memory(bytes.as_deref().unwrap_or(EMPTY))
        .warn_with("Failed to parse image, url may have pointed to a file that wasn't an image")?;

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
    Some(img)
}
