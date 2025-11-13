//! Attempt to enumerate video capture devices using Media Foundation.
//!
//! example output (first run)
/// ```notrust
/// Found 3 video capture device(s)
/// Device 0: Microsoft® LifeCam Studio(TM)
///   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0772&mi_00#a&6e1307a&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
/// Device 1: Video Camera
///   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0294&mi_00#a&2f495d5d&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
/// Device 2: icspring camera
///   Symbolic link (stable ID): \\?\usb#vid_32e6&pid_9211&mi_00#9&160b5985&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
///
///
/// example output after switching the USB ports of the last 2 devices above. the first two IDs are the same but the last is different
///
/// Device 0: Microsoft® LifeCam Studio(TM)
///   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0772&mi_00#a&6e1307a&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
/// Device 1: Video Camera
///   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0294&mi_00#a&2f495d5d&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
/// Device 2: icspring camera
///   Symbolic link (stable ID): \\?\usb#vid_32e6&pid_9211&mi_00#9&351a8e0&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
/// ```

use windows::{
    core::Result,
    Win32::{
        Foundation::{HWND, S_OK},
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
        Media::MediaFoundation::{
            MFStartup, MFShutdown,
            MFCreateAttributes,
            MFEnumDeviceSources,
            MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
            MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
            IMFAttributes, IMFActivate,
            MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
            MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
        },
    },
};
use windows::core::PWSTR;
use windows::Win32::System::Com::CoTaskMemFree;

fn main() -> Result<()> {
    unsafe {
        // Initialize COM and MF
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        MFStartup(0x0002_0070, 0)?; // use MF_VERSION macro value or 0x20070 (just example)

        // Create attributes to enumerate video capture devices
        let mut p_attrs: Option<IMFAttributes> = None;
        MFCreateAttributes(&mut p_attrs, 1)?;
        let attrs = p_attrs.as_ref().unwrap();

        attrs.SetGUID(
            &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
            &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
        )?;

        // Enumerate
        let mut p_devices_ptr: *mut Option<IMFActivate> = std::ptr::null_mut();
        let mut count: u32 = 0;
        MFEnumDeviceSources(attrs, &mut p_devices_ptr, &mut count)?;

        println!("Found {} video capture device(s)", count);
        for i in 0..count {
            let ptr = p_devices_ptr.add(i as usize);
            // Each entry is Option<IMFActivate>
            let opt_activate = &*ptr;
            if let Some(activate) = opt_activate {
                let mut pw_name = PWSTR::default();
                let mut name_len = 0u32;
                activate.GetAllocatedString(
                    &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                    &mut pw_name,
                    &mut name_len,
                )?;
                let name = pwstr_to_string(pw_name);
                CoTaskMemFree(Some(pw_name.0 as _));

                let mut pw_link = PWSTR::default();
                let mut link_len = 0u32;
                activate.GetAllocatedString(
                    &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                    &mut pw_link,
                    &mut link_len,
                )?;
                let link = pwstr_to_string(pw_link);
                CoTaskMemFree(Some(pw_link.0 as _));

                println!("Device {}: {}", i, name);
                println!("  Symbolic link (stable ID): {}", link);
            }
        }

        // Free memory
        windows::Win32::System::Com::CoTaskMemFree(Some(p_devices_ptr as *const _ as _));

        // Shutdown
        MFShutdown()?;
        CoUninitialize();
    }
    Ok(())
}


fn pwstr_to_string(pwstr: PWSTR) -> String {
    if pwstr.is_null() {
        return String::new();
    }
    unsafe { pwstr.to_string().unwrap_or_default() }
}