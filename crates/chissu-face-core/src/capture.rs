use std::fs;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use chrono::Utc;
use image::codecs::png::PngEncoder;
use image::{ColorType, GrayImage, ImageEncoder};
use tracing::debug;
use v4l::buffer::Type;
use v4l::capability::{Capabilities, Flags as CapabilityFlags};
use v4l::control::{Control, Description, Type as ControlType, Value};
use v4l::format::{Format, FourCC};
use v4l::framesize::FrameSizeEnum;
use v4l::io::mmap::Stream;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;

use crate::errors::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub device: DeviceLocator,
    pub pixel_format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub exposure: Option<i32>,
    pub gain: Option<i32>,
    pub auto_exposure: bool,
    pub auto_gain: bool,
    pub warmup_frames: u32,
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum DeviceLocator {
    Index(u32),
    Path(PathBuf),
}

impl DeviceLocator {
    pub fn from_option(device: Option<String>) -> Self {
        match device {
            Some(text) => {
                if let Ok(index) = text.parse::<u32>() {
                    DeviceLocator::Index(index)
                } else {
                    DeviceLocator::Path(PathBuf::from(text))
                }
            }
            None => DeviceLocator::Index(0),
        }
    }

    fn display(&self) -> String {
        match self {
            DeviceLocator::Index(i) => format!("/dev/video{i}"),
            DeviceLocator::Path(path) => path.display().to_string(),
        }
    }

    fn open(&self) -> Result<v4l::Device, AppError> {
        match self {
            DeviceLocator::Index(index) => {
                v4l::Device::new((*index) as usize).map_err(|err| AppError::DeviceOpen {
                    device: self.display(),
                    source: err,
                })
            }
            DeviceLocator::Path(path) => {
                v4l::Device::with_path(path).map_err(|err| AppError::DeviceOpen {
                    device: self.display(),
                    source: err,
                })
            }
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CaptureSummary {
    pub success: bool,
    pub output_path: String,
    pub device: DeviceSummary,
    pub format: NegotiatedFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gain: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_exposure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_gain: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct DeviceSummary {
    pub driver: String,
    pub card: String,
    pub bus_info: String,
    pub path: String,
}

#[derive(Debug, serde::Serialize)]
pub struct NegotiatedFormat {
    pub pixel_format: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct CaptureOutcome {
    pub summary: CaptureSummary,
    pub logs: Vec<String>,
}

#[derive(Debug)]
pub struct InMemoryCapture {
    pub image: GrayImage,
    pub device: DeviceSummary,
    pub format: NegotiatedFormat,
    pub control: CaptureControlReport,
    pub logs: Vec<String>,
}

pub fn capture_frame_in_memory(config: &CaptureConfig) -> AppResult<InMemoryCapture> {
    let mut logs = Vec::new();
    let mut device = config.device.open()?;
    logs.push(format!("Opened device {}", config.device.display()));
    debug!("device" = config.device.display(), "opened" = true);

    let caps = device.query_caps()?;
    ensure_capabilities(&caps)?;
    let device_summary = DeviceSummary {
        driver: caps.driver.clone(),
        card: caps.card.clone(),
        bus_info: caps.bus.clone(),
        path: config.device.display(),
    };
    logs.push(format!(
        "Device: driver={} card={} bus={}",
        device_summary.driver, device_summary.card, device_summary.bus_info
    ));

    let requested_fourcc = parse_fourcc(&config.pixel_format)
        .map_err(|_| AppError::UnsupportedFormat(config.pixel_format.clone()))?;
    ensure_format_supported(&device, requested_fourcc)?;
    logs.push(format!("Pixel format {} supported", config.pixel_format));

    if let Some((width, height)) = config.width.zip(config.height) {
        ensure_framesize_supported(&device, requested_fourcc, width, height, &mut logs)?;
    }

    let mut format = device.format()?;
    format.fourcc = requested_fourcc;
    if let Some(width) = config.width {
        format.width = width;
    }
    if let Some(height) = config.height {
        format.height = height;
    }
    let format = device.set_format(&format)?;
    let negotiated = NegotiatedFormat {
        pixel_format: fourcc_to_string(format.fourcc),
        width: format.width,
        height: format.height,
    };
    logs.push(format!(
        "Negotiated format: {} {}x{}",
        negotiated.pixel_format, negotiated.width, negotiated.height
    ));

    let control_report = apply_controls(&mut device, config, &mut logs)?;

    let mut stream = Stream::with_buffers(&device, Type::VideoCapture, 4)?;
    if config.warmup_frames > 0 {
        logs.push(format!(
            "Discarding {} warm-up frame(s) before capture",
            config.warmup_frames
        ));
        for idx in 0..config.warmup_frames {
            let _ = stream.next().map_err(|err| {
                AppError::FrameProcessing(format!(
                    "failed to read warm-up frame {}: {}",
                    idx + 1,
                    err
                ))
            })?;
        }
    }

    let (data, _) = stream.next()?;
    let image = convert_frame_to_image(data, &format)?;

    Ok(InMemoryCapture {
        image,
        device: device_summary,
        format: negotiated,
        control: control_report,
        logs,
    })
}

pub fn run_capture(config: &CaptureConfig) -> AppResult<CaptureOutcome> {
    let mut capture = capture_frame_in_memory(config)?;

    let output_path = ensure_output_path(config.output.as_ref())?;
    write_image(&capture.image, &output_path)?;
    capture
        .logs
        .push(format!("Saved frame to {}", output_path.display()));

    let summary = CaptureSummary {
        success: true,
        output_path: output_path.display().to_string(),
        device: capture.device,
        format: capture.format,
        exposure: config.exposure,
        gain: config.gain,
        auto_exposure: capture.control.auto_exposure,
        auto_gain: capture.control.auto_gain,
    };

    Ok(CaptureOutcome {
        summary,
        logs: capture.logs,
    })
}

fn ensure_capabilities(caps: &Capabilities) -> AppResult<()> {
    let flags = caps.capabilities;
    let mut reasons = Vec::new();
    if !flags.contains(CapabilityFlags::VIDEO_CAPTURE) {
        reasons.push("missing VIDEO_CAPTURE".to_string());
    }
    if !flags.intersects(CapabilityFlags::READ_WRITE | CapabilityFlags::STREAMING) {
        reasons.push("missing READ_WRITE or STREAMING".to_string());
    }
    if reasons.is_empty() {
        Ok(())
    } else {
        Err(AppError::Capability(reasons.join(", ")))
    }
}

fn ensure_format_supported(device: &v4l::Device, requested: FourCC) -> AppResult<()> {
    let formats = device.enum_formats()?;
    if formats.iter().any(|format| format.fourcc == requested) {
        Ok(())
    } else {
        Err(AppError::UnsupportedFormat(fourcc_to_string(requested)))
    }
}

fn ensure_framesize_supported(
    device: &v4l::Device,
    fourcc: FourCC,
    width: u32,
    height: u32,
    logs: &mut Vec<String>,
) -> AppResult<()> {
    let framesizes = device.enum_framesizes(fourcc)?;
    for size in framesizes {
        match size.size {
            FrameSizeEnum::Discrete(discrete) => {
                if discrete.width == width && discrete.height == height {
                    logs.push(format!("Frame size {width}x{height} supported (discrete)"));
                    return Ok(());
                }
            }
            FrameSizeEnum::Stepwise(step) => {
                let width_ok = width >= step.min_width && width <= step.max_width;
                let height_ok = height >= step.min_height && height <= step.max_height;
                if width_ok && height_ok {
                    logs.push(format!("Frame size {width}x{height} supported (stepwise)"));
                    return Ok(());
                }
            }
        }
    }
    Err(AppError::UnsupportedFrameSize {
        width,
        height,
        pixel_format: fourcc_to_string(fourcc),
    })
}

#[derive(Default, Debug, Clone)]
pub struct CaptureControlReport {
    pub auto_exposure: Option<String>,
    pub auto_gain: Option<String>,
}

fn apply_controls(
    device: &mut v4l::Device,
    config: &CaptureConfig,
    logs: &mut Vec<String>,
) -> AppResult<CaptureControlReport> {
    if config.exposure.is_none()
        && config.gain.is_none()
        && !config.auto_exposure
        && !config.auto_gain
    {
        return Ok(CaptureControlReport::default());
    }

    let controls = match catch_unwind_silent(|| device.query_controls()) {
        Ok(Ok(list)) => list,
        Ok(Err(err)) => {
            logs.push(format!("Unable to query controls: {err}"));
            return Ok(apply_fallback_controls(device, config, logs));
        }
        Err(_) => {
            logs.push(
                "Unable to query controls: driver reported an unsupported control type".into(),
            );
            return Ok(apply_fallback_controls(device, config, logs));
        }
    };

    let mut report = CaptureControlReport::default();
    let mut auto_exposure_active = false;
    let mut auto_gain_active = false;

    if config.auto_exposure {
        match find_control(
            &controls,
            &[
                "Exposure, Auto",
                "Exposure Auto",
                "Auto Exposure",
                "Auto_Exposure",
            ],
        ) {
            Some(ctrl) => {
                let value = auto_control_value(ctrl);
                match device.set_control(Control { id: ctrl.id, value }) {
                    Ok(_) => {
                        logs.push(format!("Enabled auto exposure via '{}'", ctrl.name));
                        report.auto_exposure = Some("applied".into());
                        auto_exposure_active = true;
                    }
                    Err(err) => {
                        logs.push(format!("Failed to enable auto exposure: {err}"));
                        report.auto_exposure = Some("failed".into());
                    }
                }
            }
            None => {
                logs.push("Auto exposure control not supported".into());
                report.auto_exposure = Some("unsupported".into());
            }
        }
    }

    if config.auto_gain {
        match find_control(
            &controls,
            &["Gain, Auto", "Auto Gain", "Gain Auto", "Gain Automatic"],
        ) {
            Some(ctrl) => {
                let value = auto_control_value(ctrl);
                match device.set_control(Control { id: ctrl.id, value }) {
                    Ok(_) => {
                        logs.push(format!("Enabled auto gain via '{}'", ctrl.name));
                        report.auto_gain = Some("applied".into());
                        auto_gain_active = true;
                    }
                    Err(err) => {
                        logs.push(format!("Failed to enable auto gain: {err}"));
                        report.auto_gain = Some("failed".into());
                    }
                }
            }
            None => {
                logs.push("Auto gain control not supported".into());
                report.auto_gain = Some("unsupported".into());
            }
        }
    }

    if let Some(exposure) = config.exposure {
        if auto_exposure_active {
            logs.push("Skipping manual exposure because auto exposure is active".into());
        } else if let Some(ctrl) = find_control(
            &controls,
            &["Exposure (Absolute)", "Exposure", "Exposure Time Absolute"],
        ) {
            match device.set_control(Control {
                id: ctrl.id,
                value: Value::Integer(exposure as i64),
            }) {
                Ok(_) => logs.push(format!("Set exposure to {exposure}")),
                Err(err) => logs.push(format!("Failed to set exposure: {err}")),
            }
        } else {
            logs.push("Exposure control not supported".into());
        }
    }

    if let Some(gain) = config.gain {
        if auto_gain_active {
            logs.push("Skipping manual gain because auto gain is active".into());
        } else if let Some(ctrl) = find_control(&controls, &["Gain"]) {
            match device.set_control(Control {
                id: ctrl.id,
                value: Value::Integer(gain as i64),
            }) {
                Ok(_) => logs.push(format!("Set gain to {gain}")),
                Err(err) => logs.push(format!("Failed to set gain: {err}")),
            }
        } else {
            logs.push("Gain control not supported".into());
        }
    }

    Ok(report)
}

fn ensure_output_path(custom: Option<&PathBuf>) -> AppResult<PathBuf> {
    let path = if let Some(path) = custom {
        path.clone()
    } else {
        let dir = Path::new("captures");
        fs::create_dir_all(dir)?;
        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
        dir.join(format!("ir-frame-{timestamp}.png"))
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(path)
}

pub fn convert_frame_to_image(data: &[u8], format: &Format) -> AppResult<GrayImage> {
    let width = format.width as usize;
    let height = format.height as usize;
    let expected_pixels = width * height;
    let pixel_format = fourcc_to_string(format.fourcc);

    match pixel_format.as_str() {
        "Y16" => {
            if data.len() < expected_pixels * 2 {
                return Err(AppError::FrameProcessing(format!(
                    "expected {} bytes for Y16 frame, got {}",
                    expected_pixels * 2,
                    data.len()
                )));
            }
            let mut buffer = Vec::with_capacity(expected_pixels);
            for idx in 0..expected_pixels {
                let low = data[idx * 2] as u16;
                let high = data[idx * 2 + 1] as u16;
                let value = (high << 8) | low;
                buffer.push((value >> 8) as u8);
            }
            GrayImage::from_vec(format.width, format.height, buffer)
                .ok_or_else(|| AppError::FrameProcessing("failed to build image buffer".into()))
        }
        "GREY" | "Y08" => {
            if data.len() < expected_pixels {
                return Err(AppError::FrameProcessing(format!(
                    "expected {} bytes for {} frame, got {}",
                    expected_pixels,
                    pixel_format,
                    data.len()
                )));
            }
            GrayImage::from_vec(
                format.width,
                format.height,
                data[..expected_pixels].to_vec(),
            )
            .ok_or_else(|| AppError::FrameProcessing("failed to build image buffer".into()))
        }
        "YUYV" => {
            let expected_bytes = expected_pixels * 2;
            if data.len() < expected_bytes {
                return Err(AppError::FrameProcessing(format!(
                    "expected {} bytes for YUYV frame, got {}",
                    expected_bytes,
                    data.len()
                )));
            }

            let mut buffer = Vec::with_capacity(expected_pixels);
            let mut chunks = data[..expected_bytes].chunks_exact(4);
            for chunk in &mut chunks {
                buffer.push(chunk[0]);
                if buffer.len() == expected_pixels {
                    break;
                }
                buffer.push(chunk[2]);
            }

            let remainder = chunks.remainder();
            if !remainder.is_empty() {
                return Err(AppError::FrameProcessing(
                    "incomplete YUYV macro-pixel encountered".into(),
                ));
            }

            GrayImage::from_vec(format.width, format.height, buffer)
                .ok_or_else(|| AppError::FrameProcessing("failed to build image buffer".into()))
        }
        other => Err(AppError::FrameProcessing(format!(
            "unsupported conversion from pixel format {other}"
        ))),
    }
}

fn write_image(image: &GrayImage, path: &Path) -> AppResult<()> {
    let mut file = std::fs::File::create(path)?;
    let encoder = PngEncoder::new(&mut file);
    encoder
        .write_image(image.as_raw(), image.width(), image.height(), ColorType::L8)
        .map_err(|err| AppError::FrameProcessing(format!("failed to encode PNG: {err}")))?;
    file.flush()?;
    Ok(())
}

fn parse_fourcc(code: &str) -> Result<FourCC, ()> {
    if code.is_empty() || code.len() > 4 {
        return Err(());
    }
    let mut repr = [b' '; 4];
    for (i, byte) in code.as_bytes().iter().enumerate() {
        repr[i] = *byte;
    }
    Ok(FourCC::new(&repr))
}

fn fourcc_to_string(fourcc: FourCC) -> String {
    String::from_utf8_lossy(&fourcc.repr)
        .trim_matches(|c| c == char::from(0) || c == ' ')
        .to_string()
}

fn find_control<'a>(controls: &'a [Description], names: &[&str]) -> Option<&'a Description> {
    controls.iter().find(|ctrl| {
        let ctrl_norm = normalize_control_name(&ctrl.name);
        names
            .iter()
            .any(|name| ctrl_norm == normalize_control_name(name))
    })
}

fn normalize_control_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_' | ','))
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn catch_unwind_silent<F, T>(f: F) -> std::thread::Result<T>
where
    F: FnOnce() -> T + panic::UnwindSafe,
{
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    panic::set_hook(hook);
    result
}

fn apply_fallback_controls(
    device: &mut v4l::Device,
    config: &CaptureConfig,
    logs: &mut Vec<String>,
) -> CaptureControlReport {
    const CID_EXPOSURE_AUTO: u32 = 0x009a0901;
    const CID_EXPOSURE_ABSOLUTE: u32 = 0x009a0902;
    const CID_GAIN: u32 = 0x00980913;
    const V4L2_EXPOSURE_APERTURE_PRIORITY: i64 = 3;

    let mut report = CaptureControlReport::default();

    logs.push("Falling back to legacy control IDs due to query failure".into());

    if config.auto_exposure {
        match device.set_control(Control {
            id: CID_EXPOSURE_AUTO,
            value: Value::Integer(V4L2_EXPOSURE_APERTURE_PRIORITY),
        }) {
            Ok(_) => {
                logs.push("Enabled auto exposure via fallback control (aperture priority)".into());
                report.auto_exposure = Some("applied".into());
            }
            Err(err) => {
                logs.push(format!(
                    "Failed to enable auto exposure via fallback control: {err}"
                ));
                report.auto_exposure = Some("failed".into());
            }
        }
    } else if config.exposure.is_some() {
        logs.push("Skipping fallback auto exposure because manual exposure requested".into());
    }

    if let Some(exposure) = config.exposure {
        match device.set_control(Control {
            id: CID_EXPOSURE_ABSOLUTE,
            value: Value::Integer(exposure as i64),
        }) {
            Ok(_) => logs.push(format!("Set exposure to {exposure} via fallback control")),
            Err(err) => logs.push(format!(
                "Failed to set exposure via fallback control: {err}"
            )),
        }
    }

    if config.auto_gain {
        logs.push("Auto gain control fallback not available in legacy path".into());
        report.auto_gain = Some("unsupported".into());
    }

    if let Some(gain) = config.gain {
        match device.set_control(Control {
            id: CID_GAIN,
            value: Value::Integer(gain as i64),
        }) {
            Ok(_) => logs.push(format!("Set gain to {gain} via fallback control")),
            Err(err) => logs.push(format!("Failed to set gain via fallback control: {err}")),
        }
    }

    report
}

fn auto_control_value(ctrl: &Description) -> Value {
    match ctrl.typ {
        ControlType::Boolean => Value::Boolean(true),
        ControlType::Menu | ControlType::Integer | ControlType::IntegerMenu => {
            let default = ctrl.default;
            let min = ctrl.minimum;
            let max = ctrl.maximum;
            let candidate = if default != min {
                default
            } else if max != min {
                max
            } else {
                min
            };
            Value::Integer(candidate)
        }
        _ => Value::Integer(ctrl.default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_format(fourcc: &str, width: u32, height: u32) -> Format {
        let fourcc = parse_fourcc(fourcc).expect("fourcc");
        Format::new(width, height, fourcc)
    }

    #[test]
    fn convert_y16_to_png_buffer() {
        let format = build_format("Y16", 2, 2);
        let data: Vec<u8> = vec![0, 0, 0, 0, 255, 255, 255, 255];
        let image = convert_frame_to_image(&data, &format).expect("convert y16");
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 2);
        assert_eq!(image.as_raw(), &vec![0, 0, 255, 255]);
    }

    #[test]
    fn convert_grey_to_png_buffer() {
        let format = build_format("GREY", 2, 2);
        let data: Vec<u8> = vec![10, 20, 30, 40];
        let image = convert_frame_to_image(&data, &format).expect("convert grey");
        assert_eq!(image.as_raw(), &data);
    }

    #[test]
    fn convert_yuyv_to_png_buffer() {
        let format = build_format("YUYV", 2, 2);
        let data: Vec<u8> = vec![10, 128, 20, 128, 30, 64, 40, 64];
        let image = convert_frame_to_image(&data, &format).expect("convert yuyv");
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 2);
        assert_eq!(image.as_raw(), &vec![10, 20, 30, 40]);
    }

    #[test]
    fn default_output_path_creates_captures_dir() {
        let unique = format!(
            "capture-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        let temp_dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
        let path = ensure_output_path(None).expect("path");
        let absolute = if path.is_absolute() {
            path.clone()
        } else {
            temp_dir.join(&path)
        };
        assert!(absolute.starts_with(temp_dir.join("captures")));
        assert_eq!(
            absolute.extension().and_then(|ext| ext.to_str()),
            Some("png")
        );
        std::env::set_current_dir(original).unwrap();
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn summary_serializes_core_fields() {
        let summary = CaptureSummary {
            success: true,
            output_path: "/tmp/captures/test.png".into(),
            device: DeviceSummary {
                driver: "uvcvideo".into(),
                card: "Test Cam".into(),
                bus_info: "usb-1".into(),
                path: "/dev/video0".into(),
            },
            format: NegotiatedFormat {
                pixel_format: "Y16".into(),
                width: 640,
                height: 480,
            },
            exposure: Some(120),
            gain: None,
            auto_exposure: None,
            auto_gain: None,
        };

        let json = serde_json::to_value(&summary).expect("serialize summary");
        assert_eq!(json["success"], true);
        assert_eq!(json["device"]["driver"], "uvcvideo");
        assert_eq!(json["format"]["pixel_format"], "Y16");
        assert_eq!(json["output_path"], "/tmp/captures/test.png");
        assert!(json["gain"].is_null());
        assert!(json["auto_exposure"].is_null());
        assert!(json["auto_gain"].is_null());
    }

    #[test]
    fn summary_includes_auto_states_when_present() {
        let summary = CaptureSummary {
            success: true,
            output_path: "/tmp/captures/test.png".into(),
            device: DeviceSummary {
                driver: "uvcvideo".into(),
                card: "Test Cam".into(),
                bus_info: "usb-1".into(),
                path: "/dev/video0".into(),
            },
            format: NegotiatedFormat {
                pixel_format: "Y16".into(),
                width: 640,
                height: 480,
            },
            exposure: None,
            gain: None,
            auto_exposure: Some("applied".into()),
            auto_gain: Some("unsupported".into()),
        };

        let json = serde_json::to_value(&summary).expect("serialize summary");
        assert_eq!(json["auto_exposure"], "applied");
        assert_eq!(json["auto_gain"], "unsupported");
    }
}
