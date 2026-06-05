#
# Rust C ABI integration.
#

set(RUST_CAPI_DIR "${ROOT_DIR}/rust")
set(RUST_CAPI_HEADER_DIR "${RUST_CAPI_DIR}/pdal-capi/include")
if(MSVC)
    set(RUST_CAPI_LIB "${RUST_CAPI_DIR}/target/release/pdal_capi.lib")
    set(RUST_CAPI_MSVC_EXPORTS
        "/EXPORT:pdal_clear_error"
        "/EXPORT:pdal_dimension_fix_name"
        "/EXPORT:pdal_dimension_interpretation_name"
        "/EXPORT:pdal_dimension_resolve_type"
        "/EXPORT:pdal_dimension_type_from_base_and_size"
        "/EXPORT:pdal_dimension_type_from_name"
        "/EXPORT:pdal_last_error"
        "/EXPORT:pdal_metadata_json_value"
        "/EXPORT:pdal_runtime_plugin_description"
        "/EXPORT:pdal_runtime_plugin_has"
        "/EXPORT:pdal_runtime_plugin_link"
        "/EXPORT:pdal_runtime_plugin_lookup_creator"
        "/EXPORT:pdal_runtime_plugin_names_json"
        "/EXPORT:pdal_runtime_plugin_register"
        "/EXPORT:pdal_string_free"
        "/EXPORT:pdal_uuid_is_null"
        "/EXPORT:pdal_uuid_parse"
        "/EXPORT:pdal_uuid_random"
        "/EXPORT:pdal_uuid_unparse"
    )
else()
    set(RUST_CAPI_LIB "${RUST_CAPI_DIR}/target/release/libpdal_capi.a")
endif()

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

# Build the pdal-capi static library with cargo and make the given PDAL library
# target depend on it. Call once, after the library target exists (the archive is
# then linked in alongside the other C++ dependencies). Keeping the cargo
# invocation here groups it with the cargo discovery and source globbing above.
macro(pdal_build_rust_capi _pdal_target)
    set(RUST_CAPI_BUILD_ENV
        MACOSX_DEPLOYMENT_TARGET=${RUST_MACOSX_DEPLOYMENT_TARGET}
    )
    if (MSVC)
        list(APPEND RUST_CAPI_BUILD_ENV
            CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=${CMAKE_LINKER}
            CXXSTDLIB=
        )
        if(CMAKE_LIBRARY_PATH)
            list(GET CMAKE_LIBRARY_PATH 0 RUST_CONDA_LIBRARY_DIR)
            file(TO_CMAKE_PATH "${RUST_CONDA_LIBRARY_DIR}" RUST_CONDA_LIBRARY_DIR)
            list(APPEND RUST_CAPI_BUILD_ENV
                PKG_CONFIG_PATH=${RUST_CONDA_LIBRARY_DIR}/pkgconfig
                GDAL_DYNAMIC=1
            )
        elseif(DEFINED ENV{CONDA_PREFIX})
            file(TO_CMAKE_PATH "$ENV{CONDA_PREFIX}" RUST_CONDA_PREFIX)
            list(APPEND RUST_CAPI_BUILD_ENV
                PKG_CONFIG_PATH=${RUST_CONDA_PREFIX}/Library/lib/pkgconfig
                GDAL_DYNAMIC=1
            )
        endif()
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
find_library(SQLITE3_LIBRARY NAMES sqlite3)
find_package(SQLite3 QUIET)

# When Nitro is available, the Rust pdal-native crate builds a NITF bridge
# (used by tools.nitfwrap, readers.nitf, and writers.nitf). The conda-forge
# Nitro headers still include POSIX headers on MSVC, so keep that bridge off for
# the MSVC build until the native shim has a Windows-compatible include path.
# Locate Nitro without polluting the global include path (nitro.cmake calls
# include_directories which would shadow vendor/nlohmann/json.hpp with a stale
# copy from the pixi env).
find_package(Nitro 2.6 QUIET MODULE)
if (NITRO_FOUND AND NOT MSVC)
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
# pdal-capi archive directly -- the main pdalcpp library and the standalone
# Rust-backed tools (lasdump, nitfwrap) -- so the Nitro/GEOS link details live
# here instead of being repeated at each call site. The archive is listed
# first so its references resolve against the native libraries that follow.
# Call after the target exists.
macro(pdal_link_rust_capi _target)
    target_link_libraries(${_target}
        PRIVATE
            ${RUST_CAPI_LIB}
            ${GEOS_C_LIBRARY}
    )
    if (TARGET SQLite3::SQLite3)
        target_link_libraries(${_target}
            PRIVATE
                SQLite3::SQLite3
        )
    elseif (TARGET SQLite::SQLite3)
        target_link_libraries(${_target}
            PRIVATE
                SQLite::SQLite3
        )
    elseif (SQLITE3_LIBRARY)
        target_link_libraries(${_target}
            PRIVATE
                ${SQLITE3_LIBRARY}
        )
    endif()
    if (APPLE)
        target_link_options(${_target} PRIVATE "SHELL:-framework CoreFoundation")
    endif()
    if (UNIX AND NOT APPLE)
        target_link_libraries(${_target}
            PRIVATE
                pthread
        )
        target_link_options(${_target} PRIVATE "-pthread")
    endif()
    if (MSVC)
        get_target_property(_target_type ${_target} TYPE)
        if (_target_type STREQUAL "SHARED_LIBRARY")
            target_link_options(${_target}
                PRIVATE
                    ${RUST_CAPI_MSVC_EXPORTS}
            )
        endif()
        target_link_libraries(${_target}
            PRIVATE
                userenv
                ntdll
        )
    endif()
    if (NITRO_FOUND AND NOT MSVC AND NITRO_LIBRARIES)
        target_link_libraries(${_target}
            PRIVATE
                ${NITRO_LIBRARIES}
        )
    endif()
endmacro()
