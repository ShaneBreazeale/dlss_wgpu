#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(unsafe_op_in_unsafe_fn)] // https://github.com/rust-lang/rust-bindgen/issues/3243

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

type PFN_vkGetDeviceProcAddr = ash::vk::PFN_vkGetDeviceProcAddr;
type PFN_vkGetInstanceProcAddr = ash::vk::PFN_vkGetInstanceProcAddr;
type VkBuffer = ash::vk::Buffer;
type VkCommandBuffer = ash::vk::CommandBuffer;
type VkDevice = ash::vk::Device;
type VkExtensionProperties = ash::vk::ExtensionProperties;
type VkFormat = ash::vk::Format;
type VkImage = ash::vk::Image;
type VkImageSubresourceRange = ash::vk::ImageSubresourceRange;
type VkImageView = ash::vk::ImageView;
type VkInstance = ash::vk::Instance;
type VkPhysicalDevice = ash::vk::PhysicalDevice;

use ash::vk::{
    ImageAspectFlags, ImageSubresourceRange, REMAINING_ARRAY_LAYERS, REMAINING_MIP_LEVELS,
};
use std::sync::atomic::{AtomicU32, Ordering};
use wgpu::{Adapter, TextureUsages, TextureView, wgc::api::Vulkan};

/// DLSS Super Resolution render preset (`NVSDK_NGX_DLSS_Hint_Render_Preset`): which trained
/// model DLSS runs. See the "Render Presets" section of the DLSS programming guide.
///
/// Applied to every [`crate::super_resolution::DlssSuperResolution`] created after
/// [`set_super_resolution_render_preset`]; an existing context keeps the preset it was
/// created with.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum DlssRenderPreset {
    /// Let DLSS pick per quality mode (may change with driver / OTA updates).
    #[default]
    Default,
    /// Similar to K; may show slightly less ghosting at the cost of extra flickering.
    J,
    /// Transformer model, the SDK default for DLAA / Quality / Balanced.
    K,
    /// The SDK default for Ultra Performance.
    L,
    /// The SDK default for Performance.
    M,
}

impl DlssRenderPreset {
    // The C enum is u32 on Linux and i32 on Windows.
    #[allow(clippy::unnecessary_cast)]
    pub(crate) fn as_ngx_value(self) -> u32 {
        (match self {
            Self::Default => {
                NVSDK_NGX_DLSS_Hint_Render_Preset_NVSDK_NGX_DLSS_Hint_Render_Preset_Default
            }
            Self::J => NVSDK_NGX_DLSS_Hint_Render_Preset_NVSDK_NGX_DLSS_Hint_Render_Preset_J,
            Self::K => NVSDK_NGX_DLSS_Hint_Render_Preset_NVSDK_NGX_DLSS_Hint_Render_Preset_K,
            Self::L => NVSDK_NGX_DLSS_Hint_Render_Preset_NVSDK_NGX_DLSS_Hint_Render_Preset_L,
            Self::M => NVSDK_NGX_DLSS_Hint_Render_Preset_NVSDK_NGX_DLSS_Hint_Render_Preset_M,
        }) as u32
    }

    fn from_ngx_value(value: u32) -> Self {
        [Self::J, Self::K, Self::L, Self::M]
            .into_iter()
            .find(|preset| preset.as_ngx_value() == value)
            .unwrap_or(Self::Default)
    }
}

static SUPER_RESOLUTION_RENDER_PRESET: AtomicU32 = AtomicU32::new(0);

/// Sets the DLSS Super Resolution render preset hint for every quality mode, process-wide.
///
/// Takes effect for contexts created afterwards.
pub fn set_super_resolution_render_preset(preset: DlssRenderPreset) {
    SUPER_RESOLUTION_RENDER_PRESET.store(preset.as_ngx_value(), Ordering::Relaxed);
}

/// The render preset hint set by [`set_super_resolution_render_preset`].
pub fn super_resolution_render_preset() -> DlssRenderPreset {
    DlssRenderPreset::from_ngx_value(SUPER_RESOLUTION_RENDER_PRESET.load(Ordering::Relaxed))
}

/// Writes the render preset hint for every quality mode into `parameters`.
pub(crate) unsafe fn apply_super_resolution_render_preset(parameters: *mut NVSDK_NGX_Parameter) {
    let value = SUPER_RESOLUTION_RENDER_PRESET.load(Ordering::Relaxed);
    for name in [
        NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_DLAA.as_slice(),
        NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_Quality.as_slice(),
        NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_Balanced.as_slice(),
        NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_Performance.as_slice(),
        NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_UltraPerformance.as_slice(),
        NVSDK_NGX_Parameter_DLSS_Hint_Render_Preset_UltraQuality.as_slice(),
    ] {
        unsafe { NVSDK_NGX_Parameter_SetUI(parameters, name.as_ptr().cast(), value) };
    }
}

/// How much DLSS should upscale by.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum DlssPerfQualityMode {
    /// Let DLSS decide.
    #[default]
    Auto,
    /// Anti-aliasing only, no upscaling.
    Dlaa,
    Quality,
    Balanced,
    Performance,
    UltraPerformance,
}

impl DlssPerfQualityMode {
    pub(crate) fn as_perf_quality_value(
        &self,
        upscaled_resolution: [u32; 2],
    ) -> NVSDK_NGX_PerfQuality_Value {
        match self {
            Self::Auto => {
                let mega_pixels =
                    (upscaled_resolution[0] * upscaled_resolution[1]) as f32 / 1_000_000.0;

                if mega_pixels < 2.03 {
                    NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_DLAA
                } else if mega_pixels < 3.68 {
                    NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_MaxQuality
                } else if mega_pixels < 8.29 {
                    NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_MaxPerf
                } else {
                    NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_UltraPerformance
                }
            }
            Self::Dlaa => NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_DLAA,
            Self::Quality => NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_MaxQuality,
            Self::Balanced => NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_Balanced,
            Self::Performance => NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_MaxPerf,
            Self::UltraPerformance => {
                NVSDK_NGX_PerfQuality_Value_NVSDK_NGX_PerfQuality_Value_UltraPerformance
            }
        }
    }
}

bitflags::bitflags! {
    /// Flags for creating a DLSS context.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DlssFeatureFlags: NVSDK_NGX_DLSS_Feature_Flags {
        /// Use an HDR texture for [`crate::DlssRenderParameters::color`] instead of an SDR texture.
        const HighDynamicRange = NVSDK_NGX_DLSS_Feature_Flags_NVSDK_NGX_DLSS_Feature_Flags_IsHDR;
        /// Motion vector values in [`crate::DlssRenderParameters::motion_vectors`] are at the render resolution.
        ///
        /// Not setting this flag indicates that the motion vectors are at the upscaled resolution and have already been dilated,
        /// which can lead to higher quality.
        ///
        /// Most renderers will want to use this flag.
        const LowResolutionMotionVectors = NVSDK_NGX_DLSS_Feature_Flags_NVSDK_NGX_DLSS_Feature_Flags_MVLowRes;
        /// Motion vector values in [`crate::DlssRenderParameters::motion_vectors`] contain jitter.
        const JitteredMotionVectors = NVSDK_NGX_DLSS_Feature_Flags_NVSDK_NGX_DLSS_Feature_Flags_MVJittered;
        /// Camera is using a reverse depth buffer for [`crate::DlssRenderParameters::depth`].
        const InvertedDepth = NVSDK_NGX_DLSS_Feature_Flags_NVSDK_NGX_DLSS_Feature_Flags_DepthInverted;
        /// Have DLSS apply auto-exposure.
        ///
        /// Not supported by [`crate::DlssRayReconstruction`].
        const AutoExposure = NVSDK_NGX_DLSS_Feature_Flags_NVSDK_NGX_DLSS_Feature_Flags_AutoExposure;
        /// Use a 4 channel RGBA texture for [`crate::DlssRenderParameters::color`] instead of a 3 channel RGB texture.
        const AlphaUpscaling = NVSDK_NGX_DLSS_Feature_Flags_NVSDK_NGX_DLSS_Feature_Flags_AlphaUpscaling;
        /// Allow DLSS to write to a subrect of [`crate::DlssRenderParameters::dlss_output`].
        const OutputSubrect = 256; // Not part of NVSDK_NGX_DLSS_Feature_Flags
    }
}

impl DlssFeatureFlags {
    pub(crate) fn as_flags(&self) -> NVSDK_NGX_DLSS_Feature_Flags {
        let mut flags = *self;
        flags.remove(DlssFeatureFlags::OutputSubrect);
        flags.bits()
    }
}

/// Errors thrown by DLSS.
#[derive(thiserror::Error, Debug)]
pub enum DlssError {
    #[error(
        "The NGX SDK or a specific feature is not supported by the current system, hardware, and/or graphics API."
    )]
    FeatureNotSupported,
    #[error(
        "An error occurred within the underlying platform, which includes the graphics API in use, the operating system, or other system libraries and dependencies that are not part of the NGX SDK, such as NvAPI. Consult the NGX logs and the graphics API's validation layers for detailed information."
    )]
    PlatformError,
    #[error(
        "The NGX feature could not be created because a feature with identical parameters already exists, and the feature does not support multiple identical instances."
    )]
    FeatureAlreadyExists,
    #[error("A feature associated with the provided handle could not be found.")]
    FeatureNotFound,
    #[error(
        "One or more provided parameters had an incorrect value or type, or a required parameter was not provided."
    )]
    InvalidParameters,
    #[error(
        "The feature requires a scratch buffer, but none was provided or the provided buffer is too small. Use NVSDK_NGX_GetScratchBufferSize to determine the necessary size."
    )]
    ScratchBufferTooSmall,
    #[error(
        "A function that requires the NGX SDK to be initialized was called before the SDK was properly initialized."
    )]
    NotInitialized,
    #[error("One or more input buffers supplied to the feature had an unsupported format.")]
    UnsupportedInputFormat,
    #[error(
        "The feature requires read/write access to output buffers, but one or more provided buffers did not have the correct access flags (UAV in D3D11/D3D12)."
    )]
    RWFlagMissing,
    #[error("A required input parameter was not provided.")]
    MissingInput,
    #[error(
        "The requested feature could not be initialized, likely because the library for that feature could not be found."
    )]
    UnableToInitializeFeature,
    #[error(
        "A function was used which requires a newer version of the NVIDIA Display Driver or feature library than is currently installed."
    )]
    OutOfDate,
    #[error("An operation could not be completed because the system lacked sufficient GPU memory.")]
    OutOfGPUMemory,
    #[error("One or more buffers provided to the feature had an unsupported format.")]
    UnsupportedFormat,
    #[error(
        "The SDK does not have the necessary write permissions for the path specified in InApplicationDataPath."
    )]
    UnableToWriteToAppDataPath,
    #[error(
        "A parameter supplied to the feature is either unsupported by the current version or has an unsupported value."
    )]
    UnsupportedParameter,
    #[error(
        "NVIDIA has restricted the use of this feature in the current application. Contact NVIDIA for further information."
    )]
    Denied,
    #[error(
        "The requested feature or functionality has not been implemented in the current version of the NGX SDK, display driver, or feature library."
    )]
    NotImplemented,
    #[error("The NGX SDK returned an unknown error code {0:#x}")]
    Unknown(NVSDK_NGX_Result),
}

pub fn check_ngx_result(result: NVSDK_NGX_Result) -> Result<(), DlssError> {
    match result {
        NVSDK_NGX_Result_NVSDK_NGX_Result_Success => Ok(()),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_FeatureNotSupported => {
            Err(DlssError::FeatureNotSupported)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_PlatformError => Err(DlssError::PlatformError),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_FeatureAlreadyExists => {
            Err(DlssError::FeatureAlreadyExists)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_FeatureNotFound => Err(DlssError::FeatureNotFound),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_InvalidParameter => {
            Err(DlssError::InvalidParameters)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_ScratchBufferTooSmall => {
            Err(DlssError::ScratchBufferTooSmall)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_NotInitialized => Err(DlssError::NotInitialized),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_UnsupportedInputFormat => {
            Err(DlssError::UnsupportedInputFormat)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_RWFlagMissing => Err(DlssError::RWFlagMissing),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_MissingInput => Err(DlssError::MissingInput),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_UnableToInitializeFeature => {
            Err(DlssError::UnableToInitializeFeature)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_OutOfDate => Err(DlssError::OutOfDate),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_OutOfGPUMemory => Err(DlssError::OutOfGPUMemory),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_UnsupportedFormat => {
            Err(DlssError::UnsupportedFormat)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_UnableToWriteToAppDataPath => {
            Err(DlssError::UnableToWriteToAppDataPath)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_UnsupportedParameter => {
            Err(DlssError::UnsupportedParameter)
        }
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_Denied => Err(DlssError::Denied),
        NVSDK_NGX_Result_NVSDK_NGX_Result_FAIL_NotImplemented => Err(DlssError::NotImplemented),
        other => Err(DlssError::Unknown(other)),
    }
}

pub fn texture_to_ngx(texture_view: &TextureView, adapter: &Adapter) -> NVSDK_NGX_Resource_VK {
    unsafe {
        let raw_view = texture_view.as_hal::<Vulkan>().unwrap().raw_handle();
        let texture = texture_view.texture();

        NVSDK_NGX_Create_ImageView_Resource_VK(
            raw_view,
            texture.as_hal::<Vulkan>().unwrap().raw_handle(),
            ImageSubresourceRange {
                aspect_mask: if texture.format().has_color_aspect() {
                    ImageAspectFlags::COLOR
                } else {
                    ImageAspectFlags::DEPTH
                },
                base_mip_level: 0,
                level_count: REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: REMAINING_ARRAY_LAYERS,
            },
            adapter
                .as_hal::<Vulkan>()
                .unwrap()
                .texture_format_as_raw(texture.format()),
            texture.width(),
            texture.height(),
            texture.usage().contains(TextureUsages::STORAGE_BINDING),
        )
    }
}

pub fn halton_sequence(mut index: u32, base: u32) -> f32 {
    let mut f = 1.0;
    let mut result = 0.0;
    while index > 0 {
        f /= base as f32;
        result += f * (index % base) as f32;
        index = (index as f32 / base as f32).floor() as u32;
    }
    result
}
