//! Attempt to enumerate video capture devices using Media Foundation.
//!
//! example output from enumerate_mf_devices (first run)
//! ```not_rust
//! Found 3 video capture device(s)
//! Device 0: Microsoft® LifeCam Studio(TM)
//!   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0772&mi_00#a&6e1307a&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
//! Device 1: Video Camera
//!   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0294&mi_00#a&2f495d5d&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
//! Device 2: icspring camera
//!   Symbolic link (stable ID): \\?\usb#vid_32e6&pid_9211&mi_00#9&160b5985&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
//!
//!
//! example output after switching the USB ports of the last 2 devices above. the first two IDs are the same but the last is different
//!
//! Device 0: Microsoft® LifeCam Studio(TM)
//!   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0772&mi_00#a&6e1307a&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
//! Device 1: Video Camera
//!   Symbolic link (stable ID): \\?\usb#vid_045e&pid_0294&mi_00#a&2f495d5d&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
//! Device 2: icspring camera
//!   Symbolic link (stable ID): \\?\usb#vid_32e6&pid_9211&mi_00#9&351a8e0&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\global
//! ```
//!
//! example output from `get_video_capture_devices`
//!
//! ```not_rust
//! Device 0: USB\VID_045E&PID_0772&MI_00\A&6E1307A&0&0000
//! Device 1: USB\VID_045E&PID_0294&MI_00\A&2F495D5D&0&0000
//! Device 2: USB\VID_32E6&PID_9211&MI_00\9&351A8E0&0&0000
//! ```
//!
//! Windows device manager reports these 'Device reported IDs hash`
//! ```not_rust
//! Video Camera: 3C94C699
//! Microsoft® LifeCam Studio(TM): 8B68F660
//! icspring camera: 8FC1CF89
//! ```
//!
//! Adding some extra code to read them results in this
//! ```not_rust
//! Device 0: USB\VID_045E&PID_0772&MI_00\A&6E1307A&0&0000
//!   Reported Device IDs hash = 8B68F660 ([96, 246, 104, 139])
//! Device 1: USB\VID_045E&PID_0294&MI_00\A&2F495D5D&0&0000
//!   Reported Device IDs hash = 3C94C699 ([153, 198, 148, 60])
//! Device 2: USB\VID_32E6&PID_9211&MI_00\9&351A8E0&0&0000
//!   Reported Device IDs hash = 8FC1CF89 ([137, 207, 193, 143])
//! ```
//!
//! If you move devices between ports, the device ID remains consistent.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::{
    core::*,
    Win32::{
        Foundation::{MAX_PATH},
        Devices::{
            DeviceAndDriverInstallation::*,
            DeviceAndDriverInstallation::SetupDiGetDevicePropertyW,
            Properties::{DEVPKEY_Device_ReportedDeviceIdsHash, DEVPROPTYPE}
        },
        System::Com::*,
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

fn main() -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).unwrap(); }

    get_video_capture_devices()?;
    enumerate_mf_devices()?;

    unsafe { CoUninitialize(); }
    Ok(())
}

fn enumerate_mf_devices() -> Result<()> {
    unsafe {
        // Initialize MF
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
    }
    Ok(())
}


fn pwstr_to_string(pwstr: PWSTR) -> String {
    if pwstr.is_null() {
        return String::new();
    }
    unsafe { pwstr.to_string().unwrap_or_default() }
}


fn get_video_capture_devices() -> Result<()> {
    unsafe {
        // Get device info set for video capture devices
        const GUID_DEVINTERFACE_VIDEO_CAPTURE: GUID = GUID::from_values(
            0xE5323777,
            0xF976,
            0x4f5b,
            [0x9B, 0x55, 0xB9, 0x46, 0x99, 0xC4, 0x6E, 0x44],
        );

        let guid = &GUID_DEVINTERFACE_VIDEO_CAPTURE;
        let hdevinfo = SetupDiGetClassDevsW(Some(guid), None, None, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE)?;

        let mut index = 0;
        loop {
            let mut device_interface_data = SP_DEVICE_INTERFACE_DATA {
                cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };

            if SetupDiEnumDeviceInterfaces(
                hdevinfo,
                None,
                guid,
                index,
                &mut device_interface_data,
            ).is_err() {
                // no more devices
                break;
            };

            // Get required buffer size
            let mut required_size = 0;
            let _ = SetupDiGetDeviceInterfaceDetailW(
                hdevinfo,
                &device_interface_data,
                None,
                0,
                Some(&mut required_size),
                None,
            );

            if required_size == 0 {
                index += 1;
                continue;
            }

            // Allocate buffer for SP_DEVICE_INTERFACE_DETAIL_DATA
            let mut buffer: Vec<u16> = vec![0; (required_size / 2) as usize];
            let detail_data = buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*detail_data).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

            let mut dev_info_data = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };

            if SetupDiGetDeviceInterfaceDetailW(
                hdevinfo,
                &device_interface_data,
                Some(detail_data),
                required_size,
                None,
                Some(&mut dev_info_data),
            ).is_err() {
                index += 1;
                continue;
            };

            // Get device instance ID (contains VID + PID + serial if available)
            let mut buf = [0u16; MAX_PATH as usize];
            let res = CM_Get_Device_IDW(
                dev_info_data.DevInst,
                &mut buf,
                0,
            );
            if res != CONFIGRET(0) {
                index += 1;
                continue;
            }

            let device_id = OsString::from_wide(&buf[..buf.iter().position(|&c| c==0).unwrap_or(buf.len())]);
            let device_id = device_id.to_string_lossy();

            println!("Device {}: {}", index, device_id);

            let mut hash_bytes = [0u8; 4]; // 4 bytes for u32
            let mut property_type = DEVPROPTYPE(0);
            let mut required_size = 0;

            if SetupDiGetDevicePropertyW(
                hdevinfo,
                &dev_info_data,
                &DEVPKEY_Device_ReportedDeviceIdsHash,
                &mut property_type,
                Some(&mut hash_bytes),
                Some(&mut required_size),
                0,
            ).is_ok() {
                let reported_hash_le = u32::from_le_bytes(hash_bytes);
                println!("  Reported Device IDs hash = {:08X} ({:?})", reported_hash_le, hash_bytes);
            } else {
                println!("  Could not read Reported Device IDs hash");
            }

            index += 1;
        }

        SetupDiDestroyDeviceInfoList(hdevinfo)?;
    }

    Ok(())
}
