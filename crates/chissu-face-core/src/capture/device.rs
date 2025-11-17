use std::path::{Path, PathBuf};

use image::GrayImage;
use v4l::buffer::Type;
use v4l::capability::Capabilities;
use v4l::control::{Control, Description};
use v4l::format::{Format, FourCC};
use v4l::framesize::FrameSize;
use v4l::io::mmap::Stream;
use v4l::io::traits::CaptureStream as V4lCaptureStream;
use v4l::video::Capture;

use crate::errors::{AppError, AppResult};

use super::{ensure_output_path, write_image, DeviceLocator};

#[cfg(test)]
use std::collections::VecDeque;
#[cfg(test)]
use v4l::capability::Flags as CapabilityFlags;
#[cfg(test)]
use v4l::framesize::FrameSizeEnum;

pub trait CaptureDevice {
    fn query_caps(&self) -> AppResult<Capabilities>;
    fn enum_formats(&self) -> AppResult<Vec<v4l::format::Description>>;
    fn enum_framesizes(&self, fourcc: FourCC) -> AppResult<Vec<FrameSize>>;
    fn format(&self) -> AppResult<Format>;
    fn set_format(&mut self, format: &Format) -> AppResult<Format>;
    fn query_controls(&self) -> AppResult<Vec<Description>>;
    fn set_control(&mut self, control: Control) -> AppResult<()>;
    fn start_stream<'a>(&'a mut self, buffer_count: u32) -> AppResult<Box<dyn CaptureStream + 'a>>;
}

pub trait CaptureStream {
    fn next(&mut self) -> AppResult<Vec<u8>>;
}

pub type CaptureDeviceFactory =
    dyn Fn(&DeviceLocator) -> AppResult<Box<dyn CaptureDevice>> + Send + Sync;

pub trait CaptureSink: Send + Sync {
    fn store(&self, image: &GrayImage, requested: Option<&Path>) -> AppResult<PathBuf>;
}

pub struct V4lCaptureDevice {
    inner: v4l::Device,
}

impl V4lCaptureDevice {
    pub fn open(locator: &DeviceLocator) -> AppResult<Self> {
        Ok(Self {
            inner: locator.open()?,
        })
    }
}

impl CaptureDevice for V4lCaptureDevice {
    fn query_caps(&self) -> AppResult<Capabilities> {
        self.inner.query_caps().map_err(AppError::from)
    }

    fn enum_formats(&self) -> AppResult<Vec<v4l::format::Description>> {
        self.inner.enum_formats().map_err(AppError::from)
    }

    fn enum_framesizes(&self, fourcc: FourCC) -> AppResult<Vec<FrameSize>> {
        self.inner.enum_framesizes(fourcc).map_err(AppError::from)
    }

    fn format(&self) -> AppResult<Format> {
        self.inner.format().map_err(AppError::from)
    }

    fn set_format(&mut self, format: &Format) -> AppResult<Format> {
        self.inner.set_format(format).map_err(AppError::from)
    }

    fn query_controls(&self) -> AppResult<Vec<Description>> {
        self.inner.query_controls().map_err(AppError::from)
    }

    fn set_control(&mut self, control: Control) -> AppResult<()> {
        self.inner.set_control(control).map_err(AppError::from)
    }

    fn start_stream<'a>(&'a mut self, buffer_count: u32) -> AppResult<Box<dyn CaptureStream + 'a>> {
        let stream = Stream::with_buffers(&self.inner, Type::VideoCapture, buffer_count)
            .map_err(AppError::from)?;
        Ok(Box::new(V4lStream { stream }))
    }
}

struct V4lStream<'a> {
    stream: Stream<'a>,
}

impl<'a> CaptureStream for V4lStream<'a> {
    fn next(&mut self) -> AppResult<Vec<u8>> {
        let (data, _) = self.stream.next().map_err(AppError::from)?;
        Ok(data.to_vec())
    }
}

pub struct FileCaptureSink;

impl CaptureSink for FileCaptureSink {
    fn store(&self, image: &GrayImage, requested: Option<&Path>) -> AppResult<PathBuf> {
        let path = ensure_output_path(requested)?;
        write_image(image, &path)?;
        Ok(path)
    }
}

#[cfg(test)]
pub struct FakeCaptureDevice {
    pub caps: Capabilities,
    pub formats: Vec<v4l::format::Description>,
    pub framesizes: Vec<FrameSize>,
    pub format: Format,
    pub control_error: Option<String>,
    pub set_control_calls: Vec<Control>,
    pub frames: VecDeque<Vec<u8>>,
}

#[cfg(test)]
impl FakeCaptureDevice {
    pub fn new(format: Format) -> Self {
        Self {
            caps: Capabilities {
                driver: "fake".into(),
                card: "fake".into(),
                bus: "loopback".into(),
                version: (0, 0, 0),
                capabilities: CapabilityFlags::empty(),
            },
            formats: Vec::new(),
            framesizes: Vec::new(),
            format,
            control_error: None,
            set_control_calls: Vec::new(),
            frames: VecDeque::new(),
        }
    }
}

#[cfg(test)]
impl CaptureDevice for FakeCaptureDevice {
    fn query_caps(&self) -> AppResult<Capabilities> {
        Ok(clone_caps(&self.caps))
    }

    fn enum_formats(&self) -> AppResult<Vec<v4l::format::Description>> {
        Ok(self.formats.iter().map(clone_description).collect())
    }

    fn enum_framesizes(&self, _fourcc: FourCC) -> AppResult<Vec<FrameSize>> {
        Ok(self.framesizes.iter().map(clone_framesize).collect())
    }

    fn format(&self) -> AppResult<Format> {
        Ok(self.format)
    }

    fn set_format(&mut self, format: &Format) -> AppResult<Format> {
        self.format = Format::new(format.width, format.height, format.fourcc);
        Ok(self.format)
    }

    fn query_controls(&self) -> AppResult<Vec<Description>> {
        if let Some(message) = &self.control_error {
            Err(AppError::FrameProcessing(message.clone()))
        } else {
            Ok(Vec::new())
        }
    }

    fn set_control(&mut self, control: Control) -> AppResult<()> {
        self.set_control_calls.push(control);
        Ok(())
    }

    fn start_stream<'a>(
        &'a mut self,
        _buffer_count: u32,
    ) -> AppResult<Box<dyn CaptureStream + 'a>> {
        Ok(Box::new(FakeStream {
            frames: &mut self.frames,
        }))
    }
}

#[cfg(test)]
struct FakeStream<'a> {
    frames: &'a mut VecDeque<Vec<u8>>,
}

#[cfg(test)]
impl<'a> CaptureStream for FakeStream<'a> {
    fn next(&mut self) -> AppResult<Vec<u8>> {
        self.frames
            .pop_front()
            .ok_or_else(|| AppError::FrameProcessing("no frame".into()))
    }
}

#[cfg(test)]
fn clone_description(desc: &v4l::format::Description) -> v4l::format::Description {
    v4l::format::Description {
        index: desc.index,
        typ: desc.typ,
        flags: desc.flags,
        description: desc.description.clone(),
        fourcc: desc.fourcc,
    }
}

#[cfg(test)]
fn clone_framesize(size: &FrameSize) -> FrameSize {
    let cloned_enum = match &size.size {
        FrameSizeEnum::Discrete(discrete) => FrameSizeEnum::Discrete(v4l::framesize::Discrete {
            width: discrete.width,
            height: discrete.height,
        }),
        FrameSizeEnum::Stepwise(step) => FrameSizeEnum::Stepwise(v4l::framesize::Stepwise {
            min_width: step.min_width,
            max_width: step.max_width,
            step_width: step.step_width,
            min_height: step.min_height,
            max_height: step.max_height,
            step_height: step.step_height,
        }),
    };

    FrameSize {
        index: size.index,
        fourcc: size.fourcc,
        typ: size.typ,
        size: cloned_enum,
    }
}

#[cfg(test)]
fn clone_caps(caps: &Capabilities) -> Capabilities {
    Capabilities {
        driver: caps.driver.clone(),
        card: caps.card.clone(),
        bus: caps.bus.clone(),
        version: caps.version,
        capabilities: caps.capabilities,
    }
}
