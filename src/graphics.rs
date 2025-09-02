use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_hal::i2c::master::I2c;
use ssd1306::{prelude::I2CInterface, size::DisplaySize128x64, Ssd1306};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Arc, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use embedded_graphics_framebuf::FrameBuf;
use embassy_time::{Duration, Timer};
use alloc::format;

extern crate alloc;

type I2cType = I2c<'static, esp_hal::Blocking>;

pub async fn boot_animation(display: &mut Ssd1306<
                I2CInterface<I2cDevice<'_, NoopRawMutex, I2cType>>,
                DisplaySize128x64,
                ssd1306::mode::BufferedGraphicsMode<DisplaySize128x64>,
            >) {
    // Create styles used by the drawing operations.
    let arc_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(5)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    let character_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();

    // Create framebuffer for 128x64 display
    const DISPLAY_WIDTH: usize = 128;
    const DISPLAY_HEIGHT: usize = 64;
    let mut framebuffer_data = [BinaryColor::Off; DISPLAY_WIDTH * DISPLAY_HEIGHT];
    let mut framebuffer = FrameBuf::new(&mut framebuffer_data, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    // The current progress percentage
    let mut progress = 0;

    loop {
        // Clear the framebuffer (off-screen)
        framebuffer.clear(BinaryColor::Off).unwrap();
        
        let sweep = progress as f32 * 360.0 / 100.0;

        // Draw to the framebuffer instead of directly to display
        Arc::new(Point::new(32, 2), 64 - 4, 90.0.deg(), sweep.deg())
            .into_styled(arc_stroke)
            .draw(&mut framebuffer).unwrap();

        // Draw centered text to framebuffer
        let text = format!("{progress}%");
        Text::with_text_style(
            &text,
            framebuffer.bounding_box().center(),
            character_style,
            text_style,
        )
        .draw(&mut framebuffer).unwrap();

        // Transfer framebuffer to display in one operation (eliminates flicker)
        let area = Rectangle::new(Point::new(0, 0), framebuffer.size());
        display.fill_contiguous(&area, framebuffer.data.iter().copied()).unwrap();
        display.flush().unwrap();

        Timer::after(Duration::from_millis(5)).await;

        progress += 1;

        if progress == 100 {
            break;
        }
    };
}
