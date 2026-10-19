use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{
    CoInitializeEx,
    COINIT_APARTMENTTHREADED,
};

pub struct VideoReader {
    reader: IMFSourceReader,
}

pub struct VideoFrame {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl VideoReader {
    pub fn open(path: &Path) -> Result<Self, String> {
        unsafe {
            let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            if result.is_err() {
                return Err(format!("COM fout: {:?}", result));
            }

            MFStartup(MF_VERSION, MFSTARTUP_FULL)
                .map_err(|e| format!("Media Foundation fout: {e}"))?;

            let path = path
                .to_str()
                .ok_or("Ongeldig bestandspad.")?;

            let wide: Vec<u16> = path
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let mut attributes = None;

            MFCreateAttributes(&mut attributes, 1)
                .map_err(|e| format!("Attributes fout: {e}"))?;

            let attributes = attributes
                .ok_or("Attributes konden niet worden aangemaakt.")?;

            attributes
                .SetUINT32(
                    &MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING,
                    1,
                )
                .map_err(|e| format!("Video processing fout: {e}"))?;

            let reader = MFCreateSourceReaderFromURL(
                PCWSTR(wide.as_ptr()),
                Some(&attributes),
            )
            .map_err(|e| format!("Video openen mislukt: {e}"))?;

            let media_type = MFCreateMediaType()
                .map_err(|e| format!("Media type fout: {e}"))?;

            media_type
                .SetGUID(
                    &MF_MT_MAJOR_TYPE,
                    &MFMediaType_Video,
                )
                .map_err(|e| format!("Major type fout: {e}"))?;

            media_type
                .SetGUID(
                    &MF_MT_SUBTYPE,
                    &MFVideoFormat_RGB32,
                )
                .map_err(|e| format!("RGB32 fout: {e}"))?;

            reader
                .SetCurrentMediaType(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    None,
                    &media_type,
                )
                .map_err(|e| format!("RGB32 instellen mislukt: {e}"))?;

            Ok(Self { reader })
        }
    }

    pub fn read_frame(&self) -> Result<VideoFrame, String> {
        unsafe {
            let mut stream = 0u32;
            let mut flags = 0u32;
            let mut timestamp = 0i64;
            let mut sample = None;

            self.reader
                .ReadSample(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    0,
                    Some(&mut stream),
                    Some(&mut flags),
                    Some(&mut timestamp),
                    Some(&mut sample),
                )
                .map_err(|e| format!("Frame lezen mislukt: {e}"))?;

            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return Err("Einde van video bereikt.".into());
            }

            let sample = sample
                .ok_or("Geen videoframe ontvangen.")?;

            let buffer = sample
                .ConvertToContiguousBuffer()
                .map_err(|e| format!("Buffer fout: {e}"))?;

            let media_type = self.reader
                .GetCurrentMediaType(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                )
                .map_err(|e| format!("Media type ophalen fout: {e}"))?;

            let size = media_type
                .GetUINT64(&MF_MT_FRAME_SIZE)
                .map_err(|e| format!("Frame size fout: {e}"))?;

            let width = (size >> 32) as usize;
            let height = (size & 0xFFFF_FFFF) as usize;

            let mut ptr = std::ptr::null_mut();
            let mut max_length = 0u32;
            let mut current_length = 0u32;

            buffer
                .Lock(
                    &mut ptr,
                    Some(&mut max_length),
                    Some(&mut current_length),
                )
                .map_err(|e| format!("Buffer lock fout: {e}"))?;

            let mut data = vec![0u8; current_length as usize];

            if !ptr.is_null() && current_length > 0 {
                std::ptr::copy_nonoverlapping(
                    ptr,
                    data.as_mut_ptr(),
                    current_length as usize,
                );
            }

            buffer
                .Unlock()
                .map_err(|e| format!("Buffer unlock fout: {e}"))?;

            Ok(VideoFrame {
                width,
                height,
                data,
            })
        }
    }
}