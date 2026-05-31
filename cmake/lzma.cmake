#
# LZMA support
#
option(WITH_LZMA
    "Build support for compression/decompression with LZMA" FALSE)
find_package(LibLZMA QUIET)
set_package_properties(LibLZMA PROPERTIES TYPE
        PURPOSE "General compression support")
if (WITH_LZMA)
    if(LIBLZMA_FOUND)
        set(CMAKE_REQUIRED_LIBRARIES ${CMAKE_REQUIRED_LIBRARIES}
            "${LIBLZMA_LIBRARIES}")
    endif()
    set(PDAL_HAVE_LZMA 1)
endif()
