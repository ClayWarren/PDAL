//! Nitro-backed NITF helpers.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::ptr;

const ERR_LEN: usize = 1024;

#[repr(C)]
struct NitfWriteOptionsRaw {
    file_title: *const c_char,
    complexity_level: *const c_char,
    system_type: *const c_char,
    origin_station_id: *const c_char,
    file_class: *const c_char,
    origin_name: *const c_char,
    origin_phone: *const c_char,
    fsclsy: *const c_char,
    fsctlh: *const c_char,
    fscltx: *const c_char,
    image_security_class: *const c_char,
    image_date_time: *const c_char,
    image_id2: *const c_char,
    aimidb: *const *const c_char,
    acftb: *const *const c_char,
    minx: c_double,
    miny: c_double,
    maxx: c_double,
    maxy: c_double,
}

type MetadataCallback =
    unsafe extern "C" fn(key: *const c_char, value: *const c_char, userdata: *mut c_void) -> c_int;

extern "C" {
    fn pdal_native_nitf_lidar_segment(
        input: *const c_char,
        offset: *mut u64,
        length: *mut u64,
        err: *mut c_char,
        err_len: usize,
    ) -> i32;

    fn pdal_native_nitf_wrap(
        input: *const c_char,
        output: *const c_char,
        title: *const c_char,
        bounds: *const c_double,
        err: *mut c_char,
        err_len: usize,
    ) -> i32;

    fn pdal_native_nitf_read_metadata(
        input: *const c_char,
        cb: MetadataCallback,
        userdata: *mut c_void,
        err: *mut c_char,
        err_len: usize,
    ) -> i32;

    fn pdal_native_nitf_write(
        input: *const c_char,
        output: *const c_char,
        opts: *const NitfWriteOptionsRaw,
        err: *mut c_char,
        err_len: usize,
    ) -> i32;
}

#[derive(Clone, Debug, Default)]
pub struct NitfWriteOptions {
    pub file_title: Option<String>,
    pub complexity_level: Option<String>,
    pub system_type: Option<String>,
    pub origin_station_id: Option<String>,
    pub file_class: Option<String>,
    pub origin_name: Option<String>,
    pub origin_phone: Option<String>,
    pub fsclsy: Option<String>,
    pub fsctlh: Option<String>,
    pub fscltx: Option<String>,
    pub image_security_class: Option<String>,
    pub image_date_time: Option<String>,
    pub image_id2: Option<String>,
    pub aimidb: Vec<String>,
    pub acftb: Vec<String>,
    pub minx: f64,
    pub miny: f64,
    pub maxx: f64,
    pub maxy: f64,
}

pub fn lidar_segment(path: &str) -> Result<(u64, u64), String> {
    let path = CString::new(path).map_err(|e| e.to_string())?;
    let mut offset = 0;
    let mut length = 0;
    let mut err = [0 as c_char; ERR_LEN];
    let ok = unsafe {
        pdal_native_nitf_lidar_segment(
            path.as_ptr(),
            &mut offset,
            &mut length,
            err.as_mut_ptr(),
            err.len(),
        )
    };
    if ok == 0 {
        return Err(take_error(&err));
    }
    Ok((offset, length))
}

unsafe extern "C" fn metadata_trampoline(
    key: *const c_char,
    value: *const c_char,
    userdata: *mut c_void,
) -> c_int {
    if userdata.is_null() || key.is_null() || value.is_null() {
        return 1;
    }
    let map = &mut *(userdata as *mut BTreeMap<String, String>);
    let key_str = match CStr::from_ptr(key).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return 0,
    };
    let value_str = CStr::from_ptr(value).to_string_lossy().into_owned();
    map.insert(key_str, value_str);
    0
}

/// Walk every NITF field/TRE and return an ordered map of `parent.tag` -> value.
pub fn read_metadata(path: &str) -> Result<BTreeMap<String, String>, String> {
    let path = CString::new(path).map_err(|e| e.to_string())?;
    let mut err = [0 as c_char; ERR_LEN];
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let userdata = &mut map as *mut BTreeMap<String, String> as *mut c_void;
    let ok = unsafe {
        pdal_native_nitf_read_metadata(
            path.as_ptr(),
            metadata_trampoline,
            userdata,
            err.as_mut_ptr(),
            err.len(),
        )
    };
    if ok == 0 {
        return Err(take_error(&err));
    }
    Ok(map)
}

/// Write a NITF file wrapping the input LAS/BPF payload, applying NITF writer
/// options (security, IDATIM, AIMIDB/ACFTB, bounds, etc.).
pub fn write(input: &str, output: &str, opts: &NitfWriteOptions) -> Result<(), String> {
    let input_c = CString::new(input).map_err(|e| e.to_string())?;
    let output_c = CString::new(output).map_err(|e| e.to_string())?;

    // Keep CStrings alive for the duration of the FFI call.
    let mut owned = Vec::<CString>::new();
    let mut take = |s: &Option<String>| -> Result<*const c_char, String> {
        match s {
            Some(value) => {
                let c = CString::new(value.as_str()).map_err(|e| e.to_string())?;
                let ptr = c.as_ptr();
                owned.push(c);
                Ok(ptr)
            }
            None => Ok(ptr::null()),
        }
    };

    let file_title = take(&opts.file_title)?;
    let complexity_level = take(&opts.complexity_level)?;
    let system_type = take(&opts.system_type)?;
    let origin_station_id = take(&opts.origin_station_id)?;
    let file_class = take(&opts.file_class)?;
    let origin_name = take(&opts.origin_name)?;
    let origin_phone = take(&opts.origin_phone)?;
    let fsclsy = take(&opts.fsclsy)?;
    let fsctlh = take(&opts.fsctlh)?;
    let fscltx = take(&opts.fscltx)?;
    let image_security_class = take(&opts.image_security_class)?;
    let image_date_time = take(&opts.image_date_time)?;
    let image_id2 = take(&opts.image_id2)?;

    let make_list = |items: &[String]| -> Result<(Vec<CString>, Vec<*const c_char>), String> {
        let mut owned = Vec::<CString>::with_capacity(items.len());
        for item in items {
            owned.push(CString::new(item.as_str()).map_err(|e| e.to_string())?);
        }
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());
        Ok((owned, ptrs))
    };
    let (_aimidb_owned, aimidb_ptrs) = make_list(&opts.aimidb)?;
    let (_acftb_owned, acftb_ptrs) = make_list(&opts.acftb)?;

    let raw = NitfWriteOptionsRaw {
        file_title,
        complexity_level,
        system_type,
        origin_station_id,
        file_class,
        origin_name,
        origin_phone,
        fsclsy,
        fsctlh,
        fscltx,
        image_security_class,
        image_date_time,
        image_id2,
        aimidb: if opts.aimidb.is_empty() {
            ptr::null()
        } else {
            aimidb_ptrs.as_ptr()
        },
        acftb: if opts.acftb.is_empty() {
            ptr::null()
        } else {
            acftb_ptrs.as_ptr()
        },
        minx: opts.minx,
        miny: opts.miny,
        maxx: opts.maxx,
        maxy: opts.maxy,
    };

    let mut err = [0 as c_char; ERR_LEN];
    let ok = unsafe {
        pdal_native_nitf_write(
            input_c.as_ptr(),
            output_c.as_ptr(),
            &raw,
            err.as_mut_ptr(),
            err.len(),
        )
    };
    drop(owned);
    drop(_aimidb_owned);
    drop(_acftb_owned);
    if ok == 0 {
        return Err(take_error(&err));
    }
    Ok(())
}

pub fn wrap(input: &str, output: &str, title: &str, bounds: [f64; 4]) -> Result<(), String> {
    let input = CString::new(input).map_err(|e| e.to_string())?;
    let output = CString::new(output).map_err(|e| e.to_string())?;
    let title = CString::new(title).map_err(|e| e.to_string())?;
    let mut err = [0 as c_char; ERR_LEN];
    let ok = unsafe {
        pdal_native_nitf_wrap(
            input.as_ptr(),
            output.as_ptr(),
            title.as_ptr(),
            bounds.as_ptr(),
            err.as_mut_ptr(),
            err.len(),
        )
    };
    if ok == 0 {
        return Err(take_error(&err));
    }
    Ok(())
}

fn take_error(err: &[c_char]) -> String {
    let bytes: Vec<u8> = err
        .iter()
        .take_while(|&&ch| ch != 0)
        .map(|&ch| ch as u8)
        .collect();
    if bytes.is_empty() {
        "NITF operation failed".to_string()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn reads_metadata_from_autzen_fixture() {
        let path = repo().join("test/data/nitf/autzen-utm10.ntf");
        let meta = read_metadata(path.to_str().unwrap()).unwrap();
        assert_eq!(
            meta.get("FH.FDT").map(String::as_str),
            Some("20120323002946")
        );
        assert_eq!(
            meta.get("IM:0.IGEOLO").map(String::as_str),
            Some("440344N1230429W440344N1230346W440300N1230346W440300N1230429W")
        );
    }

    #[test]
    fn write_round_trips_with_options() {
        let temp = tempfile::tempdir().unwrap();
        let input = repo().join("test/data/las/simple.las");
        let output = temp.path().join("opts.ntf");
        let mut opts = NitfWriteOptions::default();
        opts.file_title = Some("LiDAR from somewhere".to_string());
        opts.origin_name = Some("Howard Butler".to_string());
        opts.origin_phone = Some("5155554628".to_string());
        opts.image_date_time = Some("20110516183337".to_string());
        opts.file_class = Some("S".to_string());
        opts.minx = 0.0;
        opts.miny = 0.0;
        opts.maxx = 1.0;
        opts.maxy = 1.0;
        write(input.to_str().unwrap(), output.to_str().unwrap(), &opts).unwrap();

        let meta = read_metadata(output.to_str().unwrap()).unwrap();
        assert_eq!(
            meta.get("FH.FTITLE").map(String::as_str),
            Some("LiDAR from somewhere")
        );
        assert_eq!(
            meta.get("FH.ONAME").map(String::as_str),
            Some("Howard Butler")
        );
        assert_eq!(
            meta.get("FH.OPHONE").map(String::as_str),
            Some("5155554628")
        );
        assert_eq!(
            meta.get("IM:0.IDATIM").map(String::as_str),
            Some("20110516183337")
        );
    }

    #[test]
    fn rejects_paths_with_nul_bytes() {
        let err = lidar_segment("bad\0path").unwrap_err();
        assert!(err.contains("nul byte"));

        let err = wrap("in\0put", "out.ntf", "title", [0.0, 0.0, 1.0, 1.0]).unwrap_err();
        assert!(err.contains("nul byte"));

        let err = wrap("input.las", "out\0.ntf", "title", [0.0, 0.0, 1.0, 1.0]).unwrap_err();
        assert!(err.contains("nul byte"));

        let err = wrap("input.las", "out.ntf", "bad\0title", [0.0, 0.0, 1.0, 1.0]).unwrap_err();
        assert!(err.contains("nul byte"));
    }

    #[test]
    fn empty_native_errors_have_fallback_text() {
        assert_eq!(take_error(&[0 as c_char; ERR_LEN]), "NITF operation failed");
        let err = [b'N' as c_char, b'o' as c_char, 0, b'x' as c_char];
        assert_eq!(take_error(&err), "No");
    }
}
