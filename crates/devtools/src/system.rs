//! The system around the app in a scenario: the pickers, the save dialog and the share sheet
//! answered as the user would (`SystemAnswer`), files kept in memory, a test picture and the
//! fixture pictures, and the check that an exported file shows the canvas.

use std::sync::{Arc, Mutex, PoisonError};

use openpaint_render::RgbaImage;
use openpaint_runtime::{HostAnswer, HostRequest, ImageFormat, ImageSink, ImageSource, PortError};
use openpaint_store::{decode_image, encode_image};

use serde::Deserialize;

use crate::error::ScenarioError;

/// How the user answers what the system shows for the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemAnswer {
    /// Back or Cancel in the picker or dialog.
    Cancel,
    /// The system cannot show the picker.
    PickerFails,
    /// A place is chosen in the save dialog, or the file to share is made: the export goes
    /// into a file the runner keeps.
    ChooseFile,
    /// As `choose_file`, on storage that refuses the write.
    FullStorage,
    /// A PNG test picture (colour bands over a grey ramp) is chosen in the image picker.
    TestImage,
    /// `tests/fixtures/s26-cutout.png` is chosen in the image picker: 640 x 420, a disc on
    /// transparency with a half transparent glow.
    CutoutPng,
    /// `tests/fixtures/s26-photo.jpg` is chosen in the image picker: a camera's 1800 x 1200
    /// JPEG whose EXIF says to turn it a quarter clockwise (upright 1200 x 1800, the sun on top).
    PhotoJpeg,
    /// A text file is chosen in the image picker.
    TextFile,
    /// The share sheet shows the exported file; the user closes it with Back.
    ShareSheet,
}

/// The fixture pictures, built into the runner (it never reads files).
const CUTOUT_PNG: &[u8] = include_bytes!("../../../tests/fixtures/s26-cutout.png");
const PHOTO_JPEG: &[u8] = include_bytes!("../../../tests/fixtures/s26-photo.jpg");

/// The test picture's size: wider than tall, like a photo.
const TEST_IMAGE: [u32; 2] = [480, 320];
/// The test picture's colour bands, left to right, over its top half.
pub const TEST_BANDS: [[u8; 3]; 4] = [[220, 40, 40], [40, 170, 70], [40, 80, 220], [240, 200, 40]];
/// Mean difference per channel a JPEG may have from the canvas.
const JPEG_MEAN: f64 = 2.0;

/// The last file an export wrote, shared with the sink the runner handed out.
pub(crate) type Written = Arc<Mutex<Option<Vec<u8>>>>;

struct MemoryFile(Written);

impl ImageSink for MemoryFile {
    fn write(self: Box<Self>, bytes: &[u8]) -> Result<(), PortError> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(bytes.to_vec());
        Ok(())
    }
}

struct FullStorage;

impl ImageSink for FullStorage {
    fn write(self: Box<Self>, _bytes: &[u8]) -> Result<(), PortError> {
        Err(PortError::Device {
            detail: "no space left on the device (scenario)".into(),
        })
    }
}

struct MemorySource(Vec<u8>);

impl ImageSource for MemorySource {
    fn read(self: Box<Self>, limit: u64) -> Result<Vec<u8>, PortError> {
        let mut bytes = self.0;
        bytes.truncate(usize::try_from(limit.saturating_add(1)).unwrap_or(usize::MAX));
        Ok(bytes)
    }
}

/// Takes from `requests` the one `answer` answers and returns the platform's answer to hand to
/// the runtime; `None` for the share sheet, which answers nothing.
pub(crate) fn answer(
    requests: &mut Vec<HostRequest>,
    answer: SystemAnswer,
    written: &Written,
) -> Result<Option<HostAnswer>, ScenarioError> {
    let wanted = |request: &HostRequest| match answer {
        SystemAnswer::ShareSheet => matches!(request, HostRequest::Send { .. }),
        SystemAnswer::ChooseFile | SystemAnswer::FullStorage => matches!(
            request,
            HostRequest::SaveFile { .. } | HostRequest::ShareFile { .. }
        ),
        SystemAnswer::TestImage
        | SystemAnswer::CutoutPng
        | SystemAnswer::PhotoJpeg
        | SystemAnswer::TextFile => matches!(request, HostRequest::OpenImage),
        SystemAnswer::Cancel | SystemAnswer::PickerFails => {
            !matches!(request, HostRequest::Send { .. })
        }
    };
    let at = requests
        .iter()
        .position(wanted)
        .ok_or(ScenarioError::Missing(
            "the app asked the system for nothing this answers",
        ))?;
    requests.remove(at);
    Ok(match answer {
        SystemAnswer::ShareSheet => None,
        SystemAnswer::Cancel => Some(HostAnswer::Cancelled),
        SystemAnswer::PickerFails => Some(HostAnswer::Failed(PortError::Device {
            detail: "the picker could not be shown (scenario)".into(),
        })),
        SystemAnswer::ChooseFile => {
            Some(HostAnswer::File(Box::new(MemoryFile(Arc::clone(written)))))
        }
        SystemAnswer::FullStorage => Some(HostAnswer::File(Box::new(FullStorage))),
        SystemAnswer::TestImage => Some(HostAnswer::Image(Box::new(MemorySource(test_image()?)))),
        SystemAnswer::CutoutPng => Some(HostAnswer::Image(Box::new(MemorySource(
            CUTOUT_PNG.to_vec(),
        )))),
        SystemAnswer::PhotoJpeg => Some(HostAnswer::Image(Box::new(MemorySource(
            PHOTO_JPEG.to_vec(),
        )))),
        SystemAnswer::TextFile => Some(HostAnswer::Image(Box::new(MemorySource(
            b"Shopping list: ink, paper.\n".to_vec(),
        )))),
    })
}

/// The test picture as a PNG file: `TEST_BANDS` over the top half, a grey ramp from black to
/// white below.
fn test_image() -> Result<Vec<u8>, ScenarioError> {
    let [width, height] = TEST_IMAGE;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let [r, g, b] = if y < height / 2 {
                TEST_BANDS[(x * 4 / width) as usize]
            } else {
                let grey = (x * 255 / (width - 1)) as u8;
                [grey; 3]
            };
            pixels.extend([r, g, b, 255]);
        }
    }
    encode_image(ImageFormat::Png, TEST_IMAGE, &pixels)
        .map_err(|_| ScenarioError::Missing("the test picture could not be encoded"))
}

/// An exported file compared with the canvas it came from.
#[derive(Debug, Clone)]
pub struct Export {
    pub name: String,
    pub format: ImageFormat,
    /// The file as written.
    pub bytes: Vec<u8>,
    /// The decoded picture's size, and the canvas's.
    pub size: [u32; 2],
    pub canvas: [u32; 2],
    /// Largest and mean difference of a channel between the picture and the canvas.
    pub largest_difference: u8,
    pub mean_difference: f64,
}

impl Export {
    /// Compares `bytes` (a PNG or JPEG file) with `canvas`.
    pub(crate) fn of(
        name: &str,
        bytes: Vec<u8>,
        canvas: &RgbaImage,
    ) -> Result<Self, ScenarioError> {
        let format = if bytes.starts_with(b"\x89PNG") {
            ImageFormat::Png
        } else {
            ImageFormat::Jpeg
        };
        let (size, pixels) = decode_image(&bytes)
            .map_err(|_| ScenarioError::Missing("the exported file does not decode"))?;
        let (mut largest, mut total) = (0u8, 0u64);
        let same = size == canvas.size();
        if same {
            for (a, b) in pixels.iter().zip(canvas.pixels()) {
                let difference = a.abs_diff(*b);
                largest = largest.max(difference);
                total += u64::from(difference);
            }
        }
        let count = canvas.pixels().len().max(1) as f64;
        Ok(Self {
            name: name.to_owned(),
            format,
            bytes,
            size,
            canvas: canvas.size(),
            largest_difference: if same { largest } else { u8::MAX },
            mean_difference: if same { total as f64 / count } else { f64::MAX },
        })
    }

    /// A PNG equals the canvas pixel for pixel; a JPEG has its size and stays close.
    #[must_use]
    pub fn passed(&self) -> bool {
        match self.format {
            ImageFormat::Png => self.size == self.canvas && self.largest_difference == 0,
            ImageFormat::Jpeg => self.size == self.canvas && self.mean_difference <= JPEG_MEAN,
        }
    }
}
