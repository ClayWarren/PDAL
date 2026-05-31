
cmake_minimum_required(VERSION 3.13)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(lasdump
    Dumper.cpp
    ${ROOT_DIR}/plugins/nitf/io/tre_plugins.cpp
)

target_link_libraries(lasdump PRIVATE
    ${RUST_CAPI_LIB}
    ${PDAL_LIB_NAME}
    GDAL::GDAL
    PROJ::proj
    ${GEOS_C_LIBRARY}
    ${NITRO_LIBRARIES}
    ${COREFOUNDATION_FRAMEWORK}
)

if (LIBLZMA_FOUND)
    target_link_libraries(lasdump PRIVATE LibLZMA::LibLZMA)
endif()

target_include_directories(lasdump PRIVATE
    ${RUST_CAPI_HEADER_DIR}
    ${PROJECT_BINARY_DIR}/include
    ${ROOT_DIR}
    ${NITRO_INCLUDE_DIR}
    ${NITRO_INCLUDE_DIR}/nitro/c++
    ${NITRO_INCLUDE_DIR}/nitro/c
)

PDAL_TARGET_COMPILE_SETTINGS(lasdump)
