/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use super::RGBPixel;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct HSLPixel {
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
}

enum RGB {
    Red,
    Green,
    Blue,
}

impl RGB {
    fn from_min(r: f32, g: f32, b: f32) -> (Self, f32) {
        let (min, val) = if r < g {
            (Self::Red, r)
        } else {
            (Self::Green, g)
        };

        if val < b {
            (min, val)
        } else {
            (Self::Blue, b)
        }
    }

    fn from_max(r: f32, g: f32, b: f32) -> (Self, f32) {
        let (max, val) = if r > g {
            (Self::Red, r)
        } else {
            (Self::Green, g)
        };

        if val > b {
            (max, val)
        } else {
            (Self::Blue, b)
        }
    }
}

impl From<RGBPixel> for HSLPixel {
    fn from(rgb: RGBPixel) -> Self {
        let red = rgb.red as f32 / 255.0;
        let green = rgb.green as f32 / 255.0;
        let blue = rgb.blue as f32 / 255.0;

        let (max, max_val) = RGB::from_max(red, green, blue);
        let (_, min_val) = RGB::from_min(red, green, blue);

        let lightness = (max_val + min_val) / 2.0;

        if max_val == min_val {
            return Self {
                hue: 0.0,
                saturation: 0.0,
                lightness,
            };
        }

        let delta = max_val - min_val;

        let saturation = if lightness > 0.5 {
            delta / (2.0 - max_val - min_val)
        } else {
            delta / (max_val + min_val)
        };

        let mut hue = match max {
            RGB::Red => {
                let mut res = (green - blue) / delta;

                if green < blue {
                    res += 6.0;
                }

                res
            }
            RGB::Green => ((blue - red) / delta) + 2.0,
            RGB::Blue => ((red - green) / delta) + 4.0,
        };

        hue /= 6.0;

        Self {
            hue,
            saturation,
            lightness,
        }
    }
}
