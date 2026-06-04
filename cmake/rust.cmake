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

# Build libpdal_capi.a with cargo and make the given PDAL library target depend
# on it. Call once, after the library target exists (the archive is then linked
# in alongside the other C++ dependencies). Keeping the cargo invocation here
# groups it with the cargo discovery and source globbing above.
macro(pdal_build_rust_capi _pdal_target)
    set(RUST_CAPI_BUILD_ENV
        MACOSX_DEPLOYMENT_TARGET=${RUST_MACOSX_DEPLOYMENT_TARGET}
    )
    if (MSVC)
        list(APPEND RUST_CAPI_BUILD_ENV
            CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=${CMAKE_LINKER}
        )
    endif()
    add_custom_command(
        OUTPUT ${RUST_CAPI_LIB}
        COMMAND ${CMAKE_COMMAND} -E env
            ${RUST_CAPI_BUILD_ENV}
            ${CARGO_EXECUTABLE} build --release -p pdal-capi ${RUST_CAPI_FEATURE_ARGS}
        DEPENDS ${RUST_CAPI_SOURCES}
        WORKING_DIRECTORY ${RUST_CAPI_DIR}
        COMMENT "Building Rust C ABI (pdal-capi) with cargo"
    )
    add_custom_target(pdal_rust_capi DEPENDS ${RUST_CAPI_LIB})
    add_dependencies(${_pdal_target} pdal_rust_capi)
endmacro()

find_library(GEOS_C_LIBRARY NAMES geos_c)
if(APPLE)
    find_library(COREFOUNDATION_FRAMEWORK CoreFoundation)
endif()

# When Nitro is available, the Rust pdal-native crate builds a NITF bridge
# (used by tools.nitfwrap, readers.nitf, and writers.nitf). pdalcpp embeds
# libpdal_capi.a so it inherits those Nitro symbol references and must link the
# same native libraries. Locate them without polluting the global include path
# (nitro.cmake calls include_directories which would shadow
# vendor/nlohmann/json.hpp with a stale copy from the pixi env).
find_package(Nitro 2.6 QUIET MODULE)
if (NITRO_FOUND)
    set(RUST_CAPI_FEATURE_ARGS "--features" "nitf")
else()
    set(RUST_CAPI_FEATURE_ARGS "--no-default-features")
endif()
add_definitions("-D_REENTRANT")
if (WIN32)
    add_definitions("-DSIZEOF_SIZE_T=4")
    add_definitions("-DIMPORT_NITRO_API")
else()
    add_definitions("-D__POSIX")
endif()

# Link a target against the Rust C ABI archive and the native libraries that
# archive embeds: GEOS (via the `geos` crate), the Nitro NITF bridge, and
# CoreFoundation on Apple. Use this for every target that links
# libpdal_capi.a directly -- the main pdalcpp library and the standalone
# Rust-backed tools (lasdump, nitfwrap) -- so the Nitro/GEOS link details live
# here instead of being repeated at each call site. The archive is listed
# first so its references resolve against the native libraries that follow.
# Call after the target exists.
macro(pdal_link_rust_capi _target)
    target_link_libraries(${_target}
        PRIVATE
            ${RUST_CAPI_LIB}
            ${GEOS_C_LIBRARY}
            ${COREFOUNDATION_FRAMEWORK}
    )
    if (NITRO_FOUND AND NITRO_LIBRARIES)
        target_link_libraries(${_target}
            PRIVATE
                ${NITRO_LIBRARIES}
        )
    endif()
endmacro()
