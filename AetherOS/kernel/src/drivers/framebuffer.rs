use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use spin::Mutex;

const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;

pub struct FrameBufferWriter {
    framebuffer: &'static mut FrameBuffer,
    info: FrameBufferInfo,
    x: usize,
    y: usize,
    fg: [u8; 3],
    bg: [u8; 3],
}

impl FrameBufferWriter {
    fn new(framebuffer: &'static mut FrameBuffer) -> Self {
        let info = framebuffer.info();
        Self {
            framebuffer,
            info,
            x: 0,
            y: 0,
            fg: [0xff, 0xff, 0xff],
            bg: [0x00, 0x00, 0x00],
        }
    }

    fn clear(&mut self) {
        let buf = self.framebuffer.buffer_mut();
        for b in buf.iter_mut() {
            *b = 0;
        }
        self.x = 0;
        self.y = 0;
    }

    fn newline(&mut self) {
        self.x = 0;
        self.y += FONT_HEIGHT;
        if self.y + FONT_HEIGHT >= self.info.height {
            self.clear();
        }
    }

    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                b'\n' => self.newline(),
                byte => {
                    self.draw_glyph(byte);
                    self.x += FONT_WIDTH;
                    if self.x + FONT_WIDTH >= self.info.width {
                        self.newline();
                    }
                }
            }
        }
    }

    fn draw_glyph(&mut self, ch: u8) {
        if ch == b' ' {
            self.fill_cell(self.bg);
            return;
        }

        let mut v = ch;
        for row in 0..FONT_HEIGHT {
            let mut bits = v;
            for col in 0..FONT_WIDTH {
                let color = if (bits & 1) != 0 { self.fg } else { self.bg };
                self.set_pixel(self.x + col, self.y + row, color);
                bits = bits.rotate_left(1);
            }
            v = v.rotate_left(1);
        }
    }

    fn fill_cell(&mut self, color: [u8; 3]) {
        for row in 0..FONT_HEIGHT {
            for col in 0..FONT_WIDTH {
                self.set_pixel(self.x + col, self.y + row, color);
            }
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: [u8; 3]) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }

        let idx = (y * self.info.stride + x) * self.info.bytes_per_pixel;
        let buf = self.framebuffer.buffer_mut();

        match self.info.pixel_format {
            PixelFormat::Rgb => {
                buf[idx] = color[0];
                buf[idx + 1] = color[1];
                buf[idx + 2] = color[2];
            }
            PixelFormat::Bgr => {
                buf[idx] = color[2];
                buf[idx + 1] = color[1];
                buf[idx + 2] = color[0];
            }
            PixelFormat::U8 => {
                buf[idx] = color[0] / 3 + color[1] / 3 + color[2] / 3;
            }
            PixelFormat::Unknown { .. } => {}
        }
    }
}

static FRAMEBUFFER_WRITER: Mutex<Option<FrameBufferWriter>> = Mutex::new(None);

pub fn init(framebuffer: &'static mut FrameBuffer) {
    let mut guard = FRAMEBUFFER_WRITER.lock();
    let mut writer = FrameBufferWriter::new(framebuffer);
    writer.clear();
    *guard = Some(writer);
}

pub fn write_str(s: &str) {
    let mut guard = FRAMEBUFFER_WRITER.lock();
    if let Some(writer) = guard.as_mut() {
        writer.write_str(s);
    }
}
