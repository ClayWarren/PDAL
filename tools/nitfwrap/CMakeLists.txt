include(${ROOT_DIR}/cmake/common.cmake NO_POLICY_SCOPE)

add_executable(nitfwrap
    NitfWrap.cpp
    ${ROOT_DIR}/plugins/nitf/io/tre_plugins.cpp
)
pdal_target_compile_settings(nitfwrap)
target_link_libraries(nitfwrap PRIVATE
    ${PDAL_LIB_NAME}
    GDAL::GDAL
    PROJ::proj
)
# Rust C ABI archive + the native libraries it embeds (see cmake/rust.cmake).
pdal_link_rust_capi(nitfwrap)

target_include_directories(nitfwrap PRIVATE
    ${RUST_CAPI_HEADER_DIR}
    ${PROJECT_BINARY_DIR}/include
    ${ROOT_DIR}
    ${NITRO_INCLUDE_DIR}
    ${NITRO_INCLUDE_DIR}/nitro/c++
    ${NITRO_INCLUDE_DIR}/nitro/c
)

if (WITH_TESTS)
    PDAL_ADD_TEST(nitfwrap_test
        FILES NitfWrapTest.cpp
    )
endif()
