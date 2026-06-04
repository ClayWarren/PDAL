
cmake_minimum_required(VERSION 3.13)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(lasdump
    Dumper.cpp
)

target_link_libraries(lasdump PRIVATE
    ${PDAL_LIB_NAME}
    GDAL::GDAL
    PROJ::proj
)
# Rust C ABI archive + the native libraries it embeds (see cmake/rust.cmake).
pdal_link_rust_capi(lasdump)

if (LIBLZMA_FOUND)
    target_link_libraries(lasdump PRIVATE LibLZMA::LibLZMA)
endif()

target_include_directories(lasdump PRIVATE
    ${RUST_CAPI_HEADER_DIR}
    ${PROJECT_BINARY_DIR}/include
    ${ROOT_DIR}
)

PDAL_TARGET_COMPILE_SETTINGS(lasdump)
