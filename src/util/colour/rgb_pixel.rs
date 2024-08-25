/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use image::Rgb;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct RGBPixel {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RGBPixel {
    pub fn group(&self) -> Self {
        Self {
            red: (self.red >> 5) << 5,
            green: (self.green >> 5) << 5,
            blue: (self.blue >> 5) << 5,
        }
    }

    pub fn luminosity(&self) -> f32 {
        let red = self.red as f32 / 255.0;
        let green = self.green as f32 / 255.0;
        let blue = self.blue as f32 / 255.0;

        f32::sqrt((0.299 * (red * red)) + (0.587 * (green * green)) + (0.114 * (blue * blue)))
    }

    pub fn to_hex(self) -> u32 {
        ((self.red as u32) << 16) + ((self.green as u32) << 8) + (self.blue as u32)
    }
}

impl From<&Rgb<u8>> for RGBPixel {
    fn from(rgb: &Rgb<u8>) -> Self {
        Self {
            red: rgb.0[0],
            green: rgb.0[1],
            blue: rgb.0[2],
        }
    }
}
