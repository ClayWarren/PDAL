#
# Rust C ABI integration.
#

set(RUST_CAPI_DIR "${ROOT_DIR}/rust")
set(RUST_CAPI_HEADER_DIR "${RUST_CAPI_DIR}/pdal-capi/include")
set(RUST_CAPI_LIB "${RUST_CAPI_DIR}/target/release/libpdal_capi.a")

file(GLOB_RECURSE RUST_CAPI_SOURCES
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

find_library(GEOS_C_LIBRARY NAMES geos_c)
if(APPLE)
    find_library(COREFOUNDATION_FRAMEWORK CoreFoundation REQUIRED)
endif()

# The Rust pdal-native crate unconditionally builds a Nitro-backed NITF
# bridge (used by tools.nitfwrap, readers.nitf, and writers.nitf). pdalcpp
# embeds libpdal_capi.a so it inherits those Nitro symbol references and
# must link the same native libraries. Locate them without polluting the
# global include path (nitro.cmake calls include_directories which would
# shadow vendor/nlohmann/json.hpp with a stale copy from the pixi env).
find_package(Nitro 2.6 QUIET MODULE)
if (NOT NITRO_FOUND)
    message(FATAL_ERROR "Rust pdal-native requires Nitro >= 2.6 (set NITRO_INCLUDE_DIR / NITRO_C_LIBRARY / NITRO_CPP_LIBRARY).")
endif()
add_definitions("-D_REENTRANT")
if (WIN32)
    add_definitions("-DSIZEOF_SIZE_T=4")
    add_definitions("-DIMPORT_NITRO_API")
else()
    add_definitions("-D__POSIX")
endif()
