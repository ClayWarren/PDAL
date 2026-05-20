#
# Rust C ABI integration.
#

set(RUST_CAPI_DIR "${ROOT_DIR}/rust")
set(RUST_CAPI_HEADER_DIR "${RUST_CAPI_DIR}/pdal-capi/include")
set(RUST_CAPI_LIB "${RUST_CAPI_DIR}/target/release/libpdal_capi.a")

file(GLOB_RECURSE RUST_CAPI_SOURCES CONFIGURE_DEPENDS
    "${RUST_CAPI_DIR}/Cargo.toml"
    "${RUST_CAPI_DIR}/Cargo.lock"
    "${RUST_CAPI_DIR}/pdal-capi/*"
    "${RUST_CAPI_DIR}/pdal-cli/*"
    "${RUST_CAPI_DIR}/pdal-core/*"
    "${RUST_CAPI_DIR}/pdal-filters/*"
    "${RUST_CAPI_DIR}/pdal-io/*"
    "${RUST_CAPI_DIR}/pdal-kernels/*"
    "${RUST_CAPI_DIR}/pdal-native/*"
    "${RUST_CAPI_DIR}/pdal-plugins/*"
)

set(RUST_MACOSX_DEPLOYMENT_TARGET "${CMAKE_OSX_DEPLOYMENT_TARGET}")
if(APPLE AND NOT RUST_MACOSX_DEPLOYMENT_TARGET)
    set(RUST_MACOSX_DEPLOYMENT_TARGET "16.0")
endif()

find_program(CARGO_EXECUTABLE cargo)
if(NOT CARGO_EXECUTABLE)
    message(FATAL_ERROR "cargo (Rust) is required to build the PDAL Rust C ABI layer.")
endif()
