#
# ./apps CMake configuration file for PDAL utilties
#
# (based originally on the libLAS files copyright Mateusz Loskot)

cmake_minimum_required(VERSION 3.13)

#------------------------------------------------------------------------------
# Configure build targets
#------------------------------------------------------------------------------

set(PDAL_APP pdal)

# Kernel C ABI bridge: thin extern "C" surface over PluginManager / Kernel
# used both by the C++ pdal binary and the Rust pdal-rs binary.
add_library(pdal_kernel_capi STATIC
    "${RUST_CAPI_DIR}/pdal-capi/src/pdal_kernel_capi.cpp"
)
pdal_target_compile_settings(pdal_kernel_capi)
target_include_directories(pdal_kernel_capi PRIVATE
    ${RUST_CAPI_HEADER_DIR}
    ${PDAL_INCLUDE_DIR}
    ${PROJECT_BINARY_DIR}/include
    ${NLOHMANN_INCLUDE_DIR}
)
target_link_libraries(pdal_kernel_capi PRIVATE ${PDAL_LIB_NAME})
set_target_properties(pdal_kernel_capi PROPERTIES
    ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}"
)

if (APPLE AND PDAL_BUNDLE)
    add_executable(${PDAL_APP} MACOSX_BUNDLE pdal.cpp)
else (APPLE AND PDAL_BUNDLE)
    add_executable(${PDAL_APP} pdal.cpp)
endif(APPLE AND PDAL_BUNDLE)
pdal_target_compile_settings(${PDAL_APP})
target_link_libraries(${PDAL_APP}
    PRIVATE
        pdal_kernel_capi
        ${PDAL_LIB_NAME}
        GDAL::GDAL
        ${WINSOCK_LIBRARY}
)
target_include_directories(${PDAL_APP} PRIVATE
    ${NLOHMANN_INCLUDE_DIR}
    ${PDAL_INCLUDE_DIR}
    ${PROJECT_BINARY_DIR}/include
    ${RUST_CAPI_HEADER_DIR})

IF(MSVC)
    target_link_options(${PDAL_APP} PRIVATE /SUBSYSTEM:CONSOLE /ENTRY:wmainCRTStartup)
endif(MSVC)

if (BUILD_RUST_CLI)
    #
    # Rust CLI binary (pdal-rs)
    #
    set(PDAL_RS_APP pdal-rs)
    set(PDAL_RS_BIN "${RUST_CAPI_DIR}/target/release/pdal-rs")

    add_custom_command(
        OUTPUT ${PDAL_RS_BIN}
        COMMAND ${CMAKE_COMMAND} -E env
            MACOSX_DEPLOYMENT_TARGET=${RUST_MACOSX_DEPLOYMENT_TARGET}
            PDAL_LIB_DIR=${CMAKE_LIBRARY_OUTPUT_DIRECTORY}
            PDAL_KERNEL_CAPI_LIB=${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/libpdal_kernel_capi.a
            ${CARGO_EXECUTABLE} build -p pdal-cli --release
        DEPENDS ${RUST_CAPI_SOURCES} ${PDAL_LIB_NAME} pdal_kernel_capi
        WORKING_DIRECTORY ${RUST_CAPI_DIR}
        COMMENT "Building Rust PDAL CLI (pdal-rs) with cargo"
    )
    add_custom_target(${PDAL_RS_APP} DEPENDS ${PDAL_RS_BIN})

    add_custom_command(
        TARGET ${PDAL_RS_APP} POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy ${PDAL_RS_BIN} ${PDAL_OUTPUT_BIN_DIR}/pdal-rs
        COMMENT "Copying pdal-rs to output directory"
    )
endif()

#------------------------------------------------------------------------------
# Targets installation
#------------------------------------------------------------------------------

if (APPLE AND PDAL_BUNDLE)
    install(TARGETS ${PDAL_APP}
        BUNDLE DESTINATION ${PDAL_BIN_INSTALL_DIR})
else(APPLE AND PDAL_BUNDLE)
    install(TARGETS ${PDAL_APP}
        RUNTIME DESTINATION ${PDAL_BIN_INSTALL_DIR})
endif(APPLE AND PDAL_BUNDLE)

set(PKGCONFIG_LIBRARY_DEFINITIONS "")
set(PDAL_INCLUDE_DEFINITIONS
    "-I${CMAKE_INSTALL_PREFIX}/include")
set(PKGCONFIG_LIBRARY_DEFINITIONS "${PKGCONFIG_LIBRARY_DEFINITIONS} gdal")

if(LIBXML2_FOUND)
    set(PKGCONFIG_LIBRARY_DEFINITIONS
        "${PKGCONFIG_LIBRARY_DEFINITIONS} libxml-2.0")
    set(PDAL_INCLUDE_DEFINITIONS
        "${PDAL_INCLUDE_DEFINITIONS} -I${LIBXML2_INCLUDE_DIR}")
endif()

set(PDAL_INCLUDE_DEFINITIONS ${PDAL_INCLUDE_DEFINITIONS})

file(MAKE_DIRECTORY "${PDAL_OUTPUT_LIB_DIR}/pkgconfig/")
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/pdal.pc.in
               ${CMAKE_CURRENT_BINARY_DIR}/pdal.pc @ONLY)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/pdal.pc
    DESTINATION "${PDAL_LIB_INSTALL_DIR}/pkgconfig/"
    PERMISSIONS OWNER_READ OWNER_WRITE GROUP_READ WORLD_READ)

configure_file("${CMAKE_CURRENT_SOURCE_DIR}/pdal-config.in"
               "${PDAL_OUTPUT_BIN_DIR}/pdal-config" @ONLY
               NEWLINE_STYLE UNIX)
install(PROGRAMS "${PDAL_OUTPUT_BIN_DIR}/pdal-config"
    DESTINATION
        "${CMAKE_INSTALL_PREFIX}/bin"
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE)

if(WIN32)
    # Autoconf compatibility variables to use the same script source.
    configure_file("${CMAKE_CURRENT_SOURCE_DIR}/pdal-config-bat.in"
                   "${PDAL_OUTPUT_BIN_DIR}/pdal-config.bat" @ONLY
                   NEWLINE_STYLE CRLF)
    install(PROGRAMS "${PDAL_OUTPUT_BIN_DIR}/pdal-config.bat"
        DESTINATION
            "${CMAKE_INSTALL_PREFIX}/bin"
        PERMISSIONS
            OWNER_READ OWNER_WRITE OWNER_EXECUTE GROUP_READ GROUP_EXECUTE
            WORLD_READ WORLD_EXECUTE)
endif()
