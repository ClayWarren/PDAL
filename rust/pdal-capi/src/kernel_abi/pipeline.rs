use crate::pipeline_abi::pdal_pipeline_result_t;
use std::ffi::CStr;
use std::os::raw::c_char;

mod command;
mod info;
mod stats;

pub(in crate::kernel_abi) use command::run_pipeline_kernel;
pub(super) use info::run_info_kernel;

pub(super) unsafe fn argv_to_vec(
    argc: i32,
    argv: *const *const c_char,
) -> Result<Vec<String>, i32> {
    let mut args = Vec::new();
    for i in 0..argc {
        let arg = *argv.add(i as usize);
        if arg.is_null() {
            return Err(1);
        }
        args.push(CStr::from_ptr(arg).to_string_lossy().into_owned());
    }
    Ok(args)
}

#[allow(dead_code)]
fn _assert_result_abi_shape(_: pdal_pipeline_result_t) {}

#[cfg(test)]
mod tests;
