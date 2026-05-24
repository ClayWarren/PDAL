include(${ROOT_DIR}/cmake/common.cmake NO_POLICY_SCOPE)

add_executable(nitfwrap
    NitfWrap.cpp
    ${ROOT_DIR}/plugins/nitf/io/tre_plugins.cpp
)
pdal_target_compile_settings(nitfwrap)
target_link_libraries(nitfwrap PRIVATE
    ${RUST_CAPI_LIB}
    ${PDAL_LIB_NAME}
    GDAL::GDAL
    PROJ::proj
    ${GEOS_C_LIBRARY}
    ${NITRO_LIBRARIES}
    ${COREFOUNDATION_FRAMEWORK}
)

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
