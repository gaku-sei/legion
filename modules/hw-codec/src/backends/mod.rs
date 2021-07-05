use self::{amf::AmfEncoder, mediasdk::MediaSdkEncoder, nvenc::NvEncEncoder};
use crate::{CpuBuffer, GpuBuffer, VideoProcessor};

/// Amf Encoder/Decoder
pub mod amf;
/// `MediaSdk` Encoder/Decoder
pub mod mediasdk;
/// Null Encoder/Decoder
pub mod null;
/// `NvEnc` Encoder/Decoder
pub mod nvenc;

/// The hardware we want to run on, this maps to Amf/NvEnc/MediaSdk
#[derive(Debug, PartialEq)]
pub enum CodecHardware {
    /// Amd hardware, uses Amf library
    Amd,
    /// Nvidia hardware, uses NvEnc library
    Nvidia,
    /// Intel hardware, uses MediaSdk library
    Intel,
}

/// Graphics Context for initialization
pub enum GraphicsConfig {
    /// Vulkan config, not all values are used by all HW encoders
    Vulkan(ash::vk::Instance, ash::vk::PhysicalDevice, ash::vk::Device),
}

/// Generic configuration that applies to all encoders
/// All supported Encoder implement a conversion from the generic
/// config to the hardware specific one
pub struct EncoderConfig {
    hardware: CodecHardware,
    gfx_config: GraphicsConfig,
}

/// Generic encoder,
pub enum Encoder {
    /// Amf Encoder
    Amf(AmfEncoder),
    /// `NvEnc` Encoder
    NvEnc(NvEncEncoder),
    /// `MediaSdk` Encoder
    MediaSdk(MediaSdkEncoder),
}

impl VideoProcessor for Encoder {
    type Input = GpuBuffer;
    type Output = CpuBuffer;
    type Config = EncoderConfig;

    fn submit_input(&self, _input: &Self::Input) -> Result<(), crate::Error> {
        Ok(())
    }

    fn query_output(&self) -> Result<Self::Output, crate::Error> {
        Ok(CpuBuffer(Vec::new()))
    }

    fn new(config: Self::Config) -> Option<Self> {
        if config.hardware == CodecHardware::Amd {
            AmfEncoder::new(config.into()).map(Encoder::Amf)
        } else if config.hardware == CodecHardware::Nvidia {
            NvEncEncoder::new(config.into()).map(Encoder::NvEnc)
        } else if config.hardware == CodecHardware::Intel {
            MediaSdkEncoder::new(config.into()).map(Encoder::MediaSdk)
        } else {
            None
        }
    }
}
