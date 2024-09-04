/*
 * Copyright (c) 2021-2024 tooboredtocode
 * All Rights Reserved
 */

use std::collections::HashMap;

use super::{HslPixel, PopulatedOptions, RGBPixel};

#[derive(Debug)]
pub struct PixelGroup {
    pub group: RGBPixel,
    pub group_hsl: HslPixel,
    pub group_luminosity: f32,
    pub count: u32,
    pub pixels: HashMap<RGBPixel, u32>,
}

impl PixelGroup {
    pub fn new(group: RGBPixel) -> Self {
        Self {
            group_luminosity: group.luminosity(),
            group_hsl: group.into(),
            group,
            count: Default::default(),
            pixels: Default::default(),
        }
    }

    pub fn percentage(&self, num_pixels: u32) -> f32 {
        self.count as f32 / num_pixels as f32
    }

    pub(super) fn dom_val(&self, num_pixels: u32, options: PopulatedOptions) -> f32 {
        self.percentage(num_pixels).powf(options.percent_factor)
            * self.group_hsl.saturation.powf(options.saturation_factor)
            * self.group_luminosity.powf(options.luminosity_factor)
    }

    pub fn most_common_colour(&self) -> Option<RGBPixel> {
        self.pixels.iter().max_by(|a, b| a.1.cmp(b.1)).map(|v| *v.0)
    }
}

impl FromIterator<RGBPixel> for Vec<PixelGroup> {
    fn from_iter<T: IntoIterator<Item = RGBPixel>>(iter: T) -> Self {
        let mut res = HashMap::new();

        for pixel in iter {
            let group = pixel.group();

            let group_entry = res.entry(group).or_insert(PixelGroup::new(group));
            let pixel_count = group_entry.pixels.entry(pixel).or_insert(0);

            group_entry.count += 1;
            *pixel_count += 1;
        }

        let res: Vec<PixelGroup> = res.into_values().collect();

        res
    }
}
